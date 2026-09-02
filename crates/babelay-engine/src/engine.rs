//! 캡처 → 청커 → 전사 파이프라인 오케스트레이션.
use crate::audio::{ChunkEvent, Chunker, Resampler};
use crate::capture::{default_source, AudioSource, Frame};
use crate::transcribe::{Segment, TranscribeError, Transcriber, WhisperTranscriber};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// 큐에 오래 머문 조각의 기준(넘으면 Lagging 1회).
const LAG_THRESHOLD: Duration = Duration::from_secs(10);
/// 이 아래로 내려오면 Lagging 상태를 해제한다.
const LAG_CLEAR: Duration = Duration::from_secs(2);

pub struct EngineConfig {
    pub model_path: PathBuf,
    /// 이 세션이 쓰는 모델 id. `Started` 로 실려 나가 UI 가 실행 중인 설정을 보여준다.
    pub model_id: String,
    pub use_gpu: bool,
    /// None 이면 자동 감지.
    pub source_lang: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    Started {
        gpu_active: bool,
        gpu_fallback: bool,
        model_id: String,
        source_lang: Option<String>,
    },
    Partial {
        text: String,
        lang: String,
        start_ms: u64,
    },
    Final {
        id: u64,
        text: String,
        lang: String,
        start_ms: u64,
        end_ms: u64,
    },
    Lagging {
        queued_ms: u64,
    },
    Error {
        code: String,
        message: String,
    },
    Stopped,
}

struct Job {
    ev: ChunkEvent,
    enqueued: Instant,
}

pub struct EngineHandle {
    source: Box<dyn AudioSource>,
    frames_tx: Option<Sender<Frame>>,
    chunker: Option<JoinHandle<()>>,
    transcriber: Option<JoinHandle<()>>,
    tx: Sender<EngineEvent>,
}

impl EngineHandle {
    /// 큐를 전부 비울 때까지 블로킹한다. 진행 중인 추론 1건 + 대기 중인 최대 8건 +
    /// flush 조각을 모두 처리한 뒤에야 반환하므로, 느린 모델에서는 수십 초가 걸릴 수 있다.
    /// **UI 스레드에서 호출하면 안 된다** — 호출자가 백그라운드 스레드에서 돌려야 한다.
    /// 반환 시점에는 `Stopped` 가 정확히 한 번 발행되어 있다.
    pub fn stop(mut self) {
        self.stop_capture();
        self.drain();
    }

    /// 캡처만 즉시 멈춘다(오디오 탭 해제). 큐에 남은 조각은 그대로 두므로 빠르다.
    /// sink(= frames_tx 클론)가 드롭돼야 청커 루프가 끝난다.
    pub fn stop_capture(&mut self) {
        self.source.stop();
        drop(self.frames_tx.take());
    }

    /// 남은 큐를 다 비울 때까지 블로킹한다. `stop_capture` 뒤에 호출한다.
    pub fn drain(mut self) {
        if let Some(h) = self.chunker.take() {
            let _ = h.join();
        }
        // 청커가 끝나면 chunks_tx 가 드롭되어 전사 루프도 Stopped 를 보내고 끝난다.
        if let Some(h) = self.transcriber.take() {
            if let Err(e) = h.join() {
                // 루프가 패닉으로 끝났다면 자신의 Stopped 를 보내지 못했다. 여기서 대신 낸다.
                let _ = self.tx.send(EngineEvent::Error {
                    code: "panic".into(),
                    message: panic_msg(&e),
                });
                let _ = self.tx.send(EngineEvent::Stopped);
            }
        }
    }
}

/// 패닉 페이로드에서 사람이 읽을 메시지를 뽑는다.
fn panic_msg(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| e.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "transcriber panicked".into())
}

pub fn start_default(cfg: EngineConfig, tx: Sender<EngineEvent>) -> Result<EngineHandle, String> {
    let (t, fell_back) =
        WhisperTranscriber::load(&cfg.model_path, cfg.use_gpu).map_err(|e| e.to_string())?;
    let gpu_active = t.gpu_active;
    start(
        cfg,
        default_source(),
        Box::new(t),
        gpu_active,
        fell_back,
        tx,
    )
}

pub fn start(
    cfg: EngineConfig,
    mut source: Box<dyn AudioSource>,
    transcriber: Box<dyn Transcriber>,
    gpu_active: bool,
    gpu_fallback: bool,
    tx: Sender<EngineEvent>,
) -> Result<EngineHandle, String> {
    // ponytail: frames 는 unbounded — 과부하가 이어져도 프레임을 버리지 않고 메모리로 받는다.
    // Lagging 은 경고만 할 뿐 부하를 덜지 않는다. 실제로 메모리가 문제되면 bounded + 오래된
    // 프레임 드롭으로 바꾼다.
    let (frames_tx, frames_rx) = mpsc::channel::<Frame>();
    let (chunks_tx, chunks_rx) = mpsc::sync_channel::<Job>(8);

    // 소스를 먼저 띄운다. 실패하면 스레드를 만들지 않았으므로 고아 스레드도,
    // 유령 Stopped 도 생기지 않는다. 프레임은 채널에 쌓였다가 청커가 뜨면 소비된다.
    let ftx = frames_tx.clone();
    source
        .start(Box::new(move |f| {
            let _ = ftx.send(f);
        }))
        .map_err(|e| e.to_string())?;

    let chunker = std::thread::spawn(move || chunker_loop(frames_rx, chunks_tx));
    let tx2 = tx.clone();
    let lang = cfg.source_lang.clone();
    let transcriber_thread = std::thread::spawn(move || {
        let mut transcriber = transcriber;
        transcribe_loop(chunks_rx, &mut *transcriber, lang.as_deref(), tx2)
    });

    let _ = tx.send(EngineEvent::Started {
        gpu_active,
        gpu_fallback,
        model_id: cfg.model_id,
        source_lang: cfg.source_lang,
    });
    Ok(EngineHandle {
        source,
        frames_tx: Some(frames_tx),
        chunker: Some(chunker),
        transcriber: Some(transcriber_thread),
        tx,
    })
}

fn chunker_loop(rx: Receiver<Frame>, tx: SyncSender<Job>) {
    let mut resampler: Option<Resampler> = None;
    let mut chunker = Chunker::new();
    let mut mono = Vec::new();
    let mut warned = false;
    for f in rx {
        if f.rate == 0 || f.channels == 0 {
            if !warned {
                warned = true;
                eprintln!(
                    "babelay: 잘못된 오디오 포맷(rate {}, channels {}) — 프레임을 버린다",
                    f.rate, f.channels
                );
            }
            continue;
        }
        let r = resampler.get_or_insert_with(|| Resampler::new(f.rate, f.channels));
        mono.clear();
        r.push(&f.samples, &mut mono);
        for ev in chunker.push(&mono) {
            let partial = matches!(ev, ChunkEvent::Partial { .. });
            let job = Job {
                ev,
                enqueued: Instant::now(),
            };
            if partial {
                // 큐가 차면 Partial 은 버린다.
                let _ = tx.try_send(job);
            } else if tx.send(job).is_err() {
                return;
            }
        }
    }
    if let Some(ev) = chunker.flush() {
        let _ = tx.send(Job {
            ev,
            enqueued: Instant::now(),
        });
    }
}

fn transcribe_loop(
    rx: Receiver<Job>,
    t: &mut dyn Transcriber,
    lang: Option<&str>,
    tx: Sender<EngineEvent>,
) {
    let mut next_id = 1u64;
    let mut lagging = false;
    for job in rx {
        let waited = job.enqueued.elapsed();
        if waited > LAG_THRESHOLD && !lagging {
            lagging = true;
            let _ = tx.send(EngineEvent::Lagging {
                queued_ms: waited.as_millis() as u64,
            });
        }
        if waited < LAG_CLEAR {
            lagging = false;
        }
        match job.ev {
            ChunkEvent::Partial { pcm, start_ms } => match caught(t, &pcm, lang) {
                Ok(Ok(segs)) => {
                    if let Some(s) = segs.into_iter().next() {
                        let _ = tx.send(EngineEvent::Partial {
                            text: s.text,
                            lang: s.lang,
                            start_ms,
                        });
                    }
                }
                // Partial 의 추론 오류는 조용히 넘긴다(뒤따르는 Final 이 같은 구간을 덮는다).
                Ok(Err(_)) => {}
                Err(m) => {
                    let _ = tx.send(EngineEvent::Error {
                        code: "panic".into(),
                        message: m,
                    });
                }
            },
            ChunkEvent::Final {
                pcm,
                start_ms,
                end_ms,
            } => match caught(t, &pcm, lang) {
                Ok(Ok(segs)) => {
                    if let Some(s) = segs.into_iter().next() {
                        let _ = tx.send(EngineEvent::Final {
                            id: next_id,
                            text: s.text,
                            lang: s.lang,
                            start_ms,
                            end_ms,
                        });
                        next_id += 1;
                    }
                }
                Ok(Err(e)) => {
                    let _ = tx.send(EngineEvent::Error {
                        code: "inference".into(),
                        message: e.to_string(),
                    });
                }
                Err(m) => {
                    let _ = tx.send(EngineEvent::Error {
                        code: "panic".into(),
                        message: m,
                    });
                }
            },
        }
    }
    let _ = tx.send(EngineEvent::Stopped);
}

/// 전사기 패닉이 스레드를 죽이지 않도록 감싼다. `Err` 는 패닉 메시지.
fn caught(
    t: &mut dyn Transcriber,
    pcm: &[f32],
    lang: Option<&str>,
) -> Result<Result<Vec<Segment>, TranscribeError>, String> {
    std::panic::catch_unwind(AssertUnwindSafe(|| t.transcribe(pcm, lang)))
        .map_err(|e| panic_msg(&*e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{CaptureError, Sink};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::Duration;

    struct FakeSource {
        stop: Option<Arc<AtomicBool>>,
    }

    impl AudioSource for FakeSource {
        fn start(&mut self, mut sink: Sink) -> Result<(), CaptureError> {
            let stop = Arc::new(AtomicBool::new(false));
            let s = stop.clone();
            std::thread::spawn(move || {
                // 1.5s 톤 + 1s 무음을 48kHz 스테레오로 20ms 단위 전송.
                let mut t = 0usize;
                while !s.load(Ordering::Relaxed) && t < 125 {
                    let amp = if t < 75 { 0.3 } else { 0.0 };
                    let frame: Vec<f32> = (0..960 * 2)
                        .map(|i| amp * ((i / 2) as f32 * 0.1).sin())
                        .collect();
                    sink(Frame {
                        samples: frame,
                        rate: 48_000,
                        channels: 2,
                    });
                    t += 1;
                    std::thread::sleep(Duration::from_millis(2));
                }
            });
            self.stop = Some(stop);
            Ok(())
        }
        fn stop(&mut self) {
            if let Some(s) = &self.stop {
                s.store(true, Ordering::Relaxed);
            }
        }
    }

    struct FakeTranscriber;
    impl Transcriber for FakeTranscriber {
        fn transcribe(
            &mut self,
            pcm: &[f32],
            _lang: Option<&str>,
        ) -> Result<Vec<Segment>, TranscribeError> {
            Ok(vec![Segment {
                text: format!("{} samples", pcm.len()),
                lang: "en".into(),
                t0_ms: 0,
                t1_ms: 0,
            }])
        }
    }

    /// 첫 호출에서 패닉하고 그 뒤로는 정상 응답하는 전사기.
    struct PanickyTranscriber {
        calls: AtomicUsize,
    }
    impl Transcriber for PanickyTranscriber {
        fn transcribe(
            &mut self,
            pcm: &[f32],
            _lang: Option<&str>,
        ) -> Result<Vec<Segment>, TranscribeError> {
            if self.calls.fetch_add(1, Ordering::Relaxed) == 0 {
                panic!("boom");
            }
            Ok(vec![Segment {
                text: format!("{} samples", pcm.len()),
                lang: "en".into(),
                t0_ms: 0,
                t1_ms: 0,
            }])
        }
    }

    /// 이벤트가 조건을 만족할 때까지, 혹은 데드라인까지 모은다.
    fn drain_until(
        rx: &mpsc::Receiver<EngineEvent>,
        events: &mut Vec<EngineEvent>,
        secs: u64,
        done: impl Fn(&[EngineEvent]) -> bool,
    ) {
        let deadline = std::time::Instant::now() + Duration::from_secs(secs);
        while std::time::Instant::now() < deadline {
            if let Ok(e) = rx.recv_timeout(Duration::from_millis(200)) {
                events.push(e);
            }
            if done(events) {
                return;
            }
        }
    }

    #[test]
    fn transcriber_panic_emits_error_and_engine_still_stops() {
        let (tx, rx) = mpsc::channel();
        let cfg = EngineConfig {
            model_path: "unused".into(),
            model_id: "test-model".into(),
            use_gpu: false,
            source_lang: None,
        };
        let handle = start(
            cfg,
            Box::new(FakeSource { stop: None }),
            Box::new(PanickyTranscriber {
                calls: AtomicUsize::new(0),
            }),
            false,
            false,
            tx,
        )
        .unwrap();

        let mut events = Vec::new();
        drain_until(&rx, &mut events, 5, |ev| {
            ev.iter()
                .any(|e| matches!(e, EngineEvent::Error { code, .. } if code == "panic"))
                && ev.iter().any(|e| matches!(e, EngineEvent::Final { .. }))
        });
        handle.stop();
        drain_until(&rx, &mut events, 5, |ev| {
            matches!(ev.last(), Some(EngineEvent::Stopped))
        });

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EngineEvent::Error { code, .. } if code == "panic")),
            "패닉이 Error 이벤트로 보고되어야 한다: {events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EngineEvent::Final { .. })),
            "패닉 이후에도 다음 조각이 전사되어야 한다: {events:?}"
        );
        assert!(
            matches!(events.last(), Some(EngineEvent::Stopped)),
            "마지막 이벤트는 Stopped 여야 한다: {events:?}"
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, EngineEvent::Stopped))
                .count(),
            1,
            "Stopped 는 정확히 한 번"
        );
    }

    #[test]
    fn pipeline_emits_started_final_and_stopped() {
        let (tx, rx) = mpsc::channel();
        let cfg = EngineConfig {
            model_path: "unused".into(),
            model_id: "test-model".into(),
            use_gpu: false,
            source_lang: None,
        };
        let handle = start(
            cfg,
            Box::new(FakeSource { stop: None }),
            Box::new(FakeTranscriber),
            false,
            false,
            tx,
        )
        .unwrap();

        let mut events = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Ok(e) = rx.recv_timeout(Duration::from_millis(200)) {
                events.push(e);
            }
            if events
                .iter()
                .any(|e| matches!(e, EngineEvent::Final { .. }))
            {
                break;
            }
        }
        handle.stop();
        while let Ok(e) = rx.recv_timeout(Duration::from_secs(2)) {
            events.push(e);
            if matches!(events.last(), Some(EngineEvent::Stopped)) {
                break;
            }
        }

        assert!(matches!(events[0], EngineEvent::Started { .. }));
        let f = events
            .iter()
            .find(|e| matches!(e, EngineEvent::Final { .. }))
            .expect("a Final event");
        if let EngineEvent::Final {
            id,
            text,
            start_ms,
            end_ms,
            ..
        } = f
        {
            assert_eq!(*id, 1);
            assert!(text.ends_with("samples"));
            assert!(end_ms > start_ms);
        }
        assert!(matches!(events.last(), Some(EngineEvent::Stopped)));
    }
}
