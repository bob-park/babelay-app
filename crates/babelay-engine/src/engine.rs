//! 캡처 → 청커 → 전사 파이프라인 오케스트레이션.
use crate::audio::{ChunkEvent, Chunker, Resampler};
use crate::capture::{default_source, AudioSource, Frame};
use crate::transcribe::{Transcriber, WhisperTranscriber};
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
}

impl EngineHandle {
    pub fn stop(mut self) {
        // 캡처를 먼저 멈춰야 sink(= frames_tx 클론)가 드롭되고 청커 루프가 끝난다.
        self.source.stop();
        drop(self.frames_tx.take());
        if let Some(h) = self.chunker.take() {
            let _ = h.join();
        }
        // 청커가 끝나면 chunks_tx 가 드롭되어 전사 루프도 Stopped 를 보내고 끝난다.
        if let Some(h) = self.transcriber.take() {
            let _ = h.join();
        }
    }
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
    let (frames_tx, frames_rx) = mpsc::channel::<Frame>();
    let (chunks_tx, chunks_rx) = mpsc::sync_channel::<Job>(8);

    let chunker = std::thread::spawn(move || chunker_loop(frames_rx, chunks_tx));
    let tx2 = tx.clone();
    let lang = cfg.source_lang.clone();
    let transcriber_thread = std::thread::spawn(move || {
        let mut transcriber = transcriber;
        transcribe_loop(chunks_rx, &mut *transcriber, lang.as_deref(), tx2)
    });

    let ftx = frames_tx.clone();
    source
        .start(Box::new(move |f| {
            let _ = ftx.send(f);
        }))
        .map_err(|e| e.to_string())?;
    let _ = tx.send(EngineEvent::Started {
        gpu_active,
        gpu_fallback,
    });
    Ok(EngineHandle {
        source,
        frames_tx: Some(frames_tx),
        chunker: Some(chunker),
        transcriber: Some(transcriber_thread),
    })
}

fn chunker_loop(rx: Receiver<Frame>, tx: SyncSender<Job>) {
    let mut resampler: Option<Resampler> = None;
    let mut chunker = Chunker::new();
    let mut mono = Vec::new();
    for f in rx {
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
            ChunkEvent::Partial { pcm, start_ms } => {
                if let Ok(segs) = t.transcribe(&pcm, lang) {
                    if let Some(s) = segs.into_iter().next() {
                        let _ = tx.send(EngineEvent::Partial {
                            text: s.text,
                            lang: s.lang,
                            start_ms,
                        });
                    }
                }
            }
            ChunkEvent::Final {
                pcm,
                start_ms,
                end_ms,
            } => match t.transcribe(&pcm, lang) {
                Ok(segs) => {
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
                Err(e) => {
                    let _ = tx.send(EngineEvent::Error {
                        code: "inference".into(),
                        message: e.to_string(),
                    });
                }
            },
        }
    }
    let _ = tx.send(EngineEvent::Stopped);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{AudioSource, CaptureError, Frame, Sink};
    use crate::transcribe::{Segment, TranscribeError, Transcriber};
    use std::sync::atomic::{AtomicBool, Ordering};
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

    #[test]
    fn pipeline_emits_started_final_and_stopped() {
        let (tx, rx) = mpsc::channel();
        let cfg = EngineConfig {
            model_path: "unused".into(),
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
