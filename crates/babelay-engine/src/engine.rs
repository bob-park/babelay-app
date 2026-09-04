//! 캡처 → 청커 → 전사 → (번역) 파이프라인 오케스트레이션.
use crate::audio::{ChunkEvent, Chunker, Resampler};
use crate::capture::{default_source, AudioSource, Frame};
use crate::transcribe::{Segment, TranscribeError, Transcriber, WhisperTranscriber};
use crate::translate::{TranslateRequest, Translator};
use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// 큐에 오래 머문 조각의 기준(넘으면 Lagging 1회).
const LAG_THRESHOLD: Duration = Duration::from_secs(10);
/// 이 아래로 내려오면 Lagging 상태를 해제한다.
const LAG_CLEAR: Duration = Duration::from_secs(2);
/// 번역이 연속으로 실패할 때 Error 이벤트를 다시 내기까지의 간격.
const TRANSLATE_ERROR_INTERVAL: Duration = Duration::from_secs(30);
/// 번역 프롬프트에 붙이는 직전 원문 수.
const TRANSLATE_CONTEXT: usize = 2;

/// Final 언어 다수결 창(스펙 4단계 §3.1). Whisper 는 Final 마다 언어를 새로 감지하고 짧은
/// 발화에서 자주 틀린다 — 한 번의 오감지가 번역 호출(또는 패스쓰루 누락)로 이어지지 않게 한다.
const LANG_WINDOW: usize = 3;

/// 최근 `LANG_WINDOW` 개 Final 의 감지 언어 다수결. 동률이면 이번 감지값.
/// ponytail: 언어가 세션 중 실제로 바뀌면 Final 2개만큼 늦게 따라간다. 더 빨리 따라가야 하면 창을 줄인다.
struct LangVote(VecDeque<String>);

impl LangVote {
    fn new() -> Self {
        Self(VecDeque::with_capacity(LANG_WINDOW + 1))
    }

    /// 이번 감지값을 넣고 확정 언어를 돌려준다.
    fn push(&mut self, detected: String) -> String {
        self.0.push_back(detected.clone());
        if self.0.len() > LANG_WINDOW {
            self.0.pop_front();
        }
        let count = |l: &str| self.0.iter().filter(|x| x.as_str() == l).count();
        let mine = count(&detected);
        self.0
            .iter()
            .find(|l| count(l) > mine)
            .cloned()
            .unwrap_or(detected)
    }
}

pub struct EngineConfig {
    pub model_path: PathBuf,
    /// 이 세션이 쓰는 모델 id. `Started` 로 실려 나가 UI 가 실행 중인 설정을 보여준다.
    pub model_id: String,
    pub use_gpu: bool,
    /// None 이면 자동 감지.
    pub source_lang: Option<String>,
    /// 번역 타겟 언어. None 이면 번역하지 않는다(번역기가 있어도 스레드를 만들지 않는다).
    pub tgt_lang: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    Started {
        gpu_active: bool,
        gpu_fallback: bool,
        model_id: String,
        source_lang: Option<String>,
        /// 번역 타겟 코드. 번역기가 붙어 있을 때만 `Some` — 프론트가 타겟을 다시 유추하지 않는다.
        target_lang: Option<String>,
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
    /// `Final{id}` 의 번역. 원어가 타겟과 같으면 발행되지 않는다.
    Translated {
        id: u64,
        text: String,
        lang: String,
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

/// 전사 스레드 → 번역 스레드로 넘기는 확정 문장 (id, 원문, 원어).
type TranslateJob = (u64, String, String);

pub struct EngineHandle {
    source: Box<dyn AudioSource>,
    frames_tx: Option<Sender<Frame>>,
    chunker: Option<JoinHandle<()>>,
    transcriber: Option<JoinHandle<()>>,
    translator: Option<JoinHandle<()>>,
    /// 정지 신호. 켜지면 번역 루프는 남은 큐를 번역하지 않고 흘려보낸다.
    discard: Arc<AtomicBool>,
    tx: Sender<EngineEvent>,
}

impl EngineHandle {
    /// 큐를 전부 비울 때까지 블로킹한다. 진행 중인 추론 1건 + 대기 중인 최대 8건 +
    /// flush 조각을 모두 처리한 뒤에야 반환하므로, 느린 모델에서는 수십 초가 걸릴 수 있다.
    /// 번역은 기다리지 않는다(`stop_capture` 가 대기 큐를 버린다).
    /// **UI 스레드에서 호출하면 안 된다** — 호출자가 백그라운드 스레드에서 돌려야 한다.
    /// 반환 시점에는 `Stopped` 가 정확히 한 번 발행되어 있다.
    pub fn stop(mut self) {
        self.stop_capture();
        self.drain();
    }

    /// 캡처만 즉시 멈춘다(오디오 탭 해제). 큐에 남은 조각은 그대로 두므로 빠르다.
    /// sink(= frames_tx 클론)가 드롭돼야 청커 루프가 끝난다.
    /// 대기 중인 번역은 여기서 포기한다 — 세션이 끝난 뒤의 번역은 화면에 쓸 데가 없고,
    /// 재시도 예산(조각당 최대 ~31초)을 다 물면 정지가 몇 분씩 걸린다.
    pub fn stop_capture(&mut self) {
        self.discard.store(true, Ordering::Relaxed);
        self.source.stop();
        drop(self.frames_tx.take());
    }

    /// 남은 큐를 다 비울 때까지 블로킹한다. `stop_capture` 뒤에 호출한다.
    pub fn drain(mut self) {
        if let Some(h) = self.chunker.take() {
            let _ = h.join();
        }
        // 청커가 끝나면 chunks_tx 가 드롭되어 전사 루프가 끝나고, 전사 루프가 끝나면 번역 큐
        // 송신단이 드롭되어 번역 루프도 끝난다. Stopped 는 마지막 스레드가 보낸다.
        let transcriber_panic = self.transcriber.take().and_then(|h| h.join().err());
        // Some(None) = 번역 스레드가 정상 종료(Stopped 를 보냈다), Some(Some(_)) = 패닉, None = 번역 단계 없음.
        let translator_panic = self.translator.take().map(|h| h.join().err());
        if let Some(e) = &transcriber_panic {
            let _ = self.tx.send(EngineEvent::Error {
                code: "panic".into(),
                message: panic_msg(&**e),
            });
        }
        // 마지막 스레드가 패닉으로 끝났다면 자신의 Stopped 를 보내지 못했다. 여기서 대신 낸다.
        let stopped_lost = match translator_panic {
            Some(Some(e)) => {
                let _ = self.tx.send(EngineEvent::Error {
                    code: "panic".into(),
                    message: panic_msg(&*e),
                });
                true
            }
            Some(None) => false,
            None => transcriber_panic.is_some(),
        };
        if stopped_lost {
            let _ = self.tx.send(EngineEvent::Stopped);
        }
    }
}

/// 패닉 페이로드에서 사람이 읽을 메시지를 뽑는다.
fn panic_msg(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| e.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "worker panicked".into())
}

pub fn start_default(
    cfg: EngineConfig,
    translator: Option<Box<dyn Translator>>,
    tx: Sender<EngineEvent>,
) -> Result<EngineHandle, String> {
    let (t, fell_back) =
        WhisperTranscriber::load(&cfg.model_path, cfg.use_gpu).map_err(|e| e.to_string())?;
    let gpu_active = t.gpu_active;
    start(
        cfg,
        default_source(),
        Box::new(t),
        translator,
        gpu_active,
        fell_back,
        tx,
    )
}

pub fn start(
    cfg: EngineConfig,
    mut source: Box<dyn AudioSource>,
    transcriber: Box<dyn Transcriber>,
    translator: Option<Box<dyn Translator>>,
    gpu_active: bool,
    gpu_fallback: bool,
    tx: Sender<EngineEvent>,
) -> Result<EngineHandle, String> {
    // ponytail: frames 는 unbounded — 과부하가 이어져도 프레임을 버리지 않고 메모리로 받는다.
    // Lagging 은 경고만 할 뿐 부하를 덜지 않는다. 실제로 메모리가 문제되면 bounded + 오래된
    // 프레임 드롭으로 바꾼다.
    let (frames_tx, frames_rx) = mpsc::channel::<Frame>();
    let (chunks_tx, chunks_rx) = mpsc::sync_channel::<Job>(8);
    // 번역기와 타겟이 모두 있을 때만 번역 단계를 만든다.
    let translation = match (translator, cfg.tgt_lang.clone()) {
        (Some(t), Some(tgt)) => Some((t, tgt)),
        _ => None,
    };
    let target_lang = translation.as_ref().map(|(_, tgt)| tgt.clone());
    let discard = Arc::new(AtomicBool::new(false));

    // 소스를 먼저 띄운다. 실패하면 스레드를 만들지 않았으므로 고아 스레드도,
    // 유령 Stopped 도 생기지 않는다. 프레임은 채널에 쌓였다가 청커가 뜨면 소비된다.
    let ftx = frames_tx.clone();
    source
        .start(Box::new(move |f| {
            let _ = ftx.send(f);
        }))
        .map_err(|e| e.to_string())?;

    let chunker = std::thread::spawn(move || chunker_loop(frames_rx, chunks_tx));

    let (translate_tx, translator_thread) = match translation {
        Some((t, tgt)) => {
            let (ttx, trx) = mpsc::sync_channel::<TranslateJob>(16);
            let tx3 = tx.clone();
            let d = discard.clone();
            let h = std::thread::spawn(move || translate_loop(trx, t, tgt, tx3, d));
            (Some(ttx), Some(h))
        }
        None => (None, None),
    };

    let tx2 = tx.clone();
    let lang = cfg.source_lang.clone();
    let emit_stopped = translator_thread.is_none();
    let transcriber_thread = std::thread::spawn(move || {
        let mut transcriber = transcriber;
        transcribe_loop(
            chunks_rx,
            &mut *transcriber,
            lang.as_deref(),
            tx2,
            translate_tx,
            emit_stopped,
        )
    });

    let _ = tx.send(EngineEvent::Started {
        gpu_active,
        gpu_fallback,
        model_id: cfg.model_id,
        source_lang: cfg.source_lang,
        target_lang,
    });
    Ok(EngineHandle {
        source,
        frames_tx: Some(frames_tx),
        chunker: Some(chunker),
        transcriber: Some(transcriber_thread),
        translator: translator_thread,
        discard,
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
    translate_tx: Option<SyncSender<TranslateJob>>,
    emit_stopped: bool,
) {
    let mut next_id = 1u64;
    let mut lagging = false;
    let mut vote = LangVote::new();
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
                        let id = next_id;
                        next_id += 1;
                        // 감지값 하나가 아니라 최근 Final 들의 다수결로 원어를 정한다(오감지 완화).
                        let lang = vote.push(s.lang);
                        let job = translate_tx
                            .as_ref()
                            .map(|_| (id, s.text.clone(), lang.clone()));
                        // 원문이 먼저다. 번역 큐 사정이 자막을 늦추면 안 된다.
                        let _ = tx.send(EngineEvent::Final {
                            id,
                            text: s.text,
                            lang,
                            start_ms,
                            end_ms,
                        });
                        // 큐가 차면 그 조각은 버린다 — 원문만 보이고 전사는 계속 흐른다.
                        if let (Some(q), Some(job)) = (&translate_tx, job) {
                            let _ = q.try_send(job);
                        }
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
    // 송신단을 놓아야 번역 루프가 끝난다. 번역 스레드가 있으면 Stopped 는 그쪽이 보낸다.
    drop(translate_tx);
    if emit_stopped {
        let _ = tx.send(EngineEvent::Stopped);
    }
}

fn translate_loop(
    rx: Receiver<TranslateJob>,
    mut translator: Box<dyn Translator>,
    tgt: String,
    tx: Sender<EngineEvent>,
    discard: Arc<AtomicBool>,
) {
    let mut context: VecDeque<String> = VecDeque::with_capacity(TRANSLATE_CONTEXT + 1);
    let mut last_error: Option<Instant> = None;
    for (id, text, lang) in rx {
        // 정지 신호가 왔으면 남은 큐는 번역하지 않고 비운다.
        if discard.load(Ordering::Relaxed) {
            continue;
        }
        if lang != tgt {
            let req = TranslateRequest {
                text: text.clone(),
                src: lang,
                tgt: tgt.clone(),
                context: context.iter().cloned().collect(),
            };
            match std::panic::catch_unwind(AssertUnwindSafe(|| translator.translate(&req))) {
                Ok(Ok(out)) => {
                    last_error = None;
                    let _ = tx.send(EngineEvent::Translated {
                        id,
                        text: out,
                        lang: tgt.clone(),
                    });
                }
                Ok(Err(e)) => {
                    eprintln!("babelay: 번역 실패({}) id={id}: {e}", translator.name());
                    let due = last_error.is_none_or(|t| t.elapsed() >= TRANSLATE_ERROR_INTERVAL);
                    if due {
                        last_error = Some(Instant::now());
                        let _ = tx.send(EngineEvent::Error {
                            code: "translate".into(),
                            message: e.to_string(),
                        });
                    }
                }
                Err(p) => {
                    let _ = tx.send(EngineEvent::Error {
                        code: "panic".into(),
                        message: panic_msg(&*p),
                    });
                }
            }
        }
        context.push_back(text);
        if context.len() > TRANSLATE_CONTEXT {
            context.pop_front();
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
    use crate::translate::TranslateError;
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

    struct UpperTranslator;
    impl Translator for UpperTranslator {
        fn name(&self) -> &str {
            "upper"
        }
        fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
            Ok(req.text.to_uppercase())
        }
    }

    /// 요청마다 1초를 태우는 번역기. 정지 시 큐를 버리지 않으면 테스트가 느려진다.
    struct SlowTranslator;
    impl Translator for SlowTranslator {
        fn name(&self) -> &str {
            "slow"
        }
        fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
            std::thread::sleep(Duration::from_secs(1));
            Ok(req.text.clone())
        }
    }

    /// 받은 요청의 context 를 그대로 기록한다.
    struct RecordingTranslator(Arc<std::sync::Mutex<Vec<Vec<String>>>>);
    impl Translator for RecordingTranslator {
        fn name(&self) -> &str {
            "recording"
        }
        fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
            self.0.lock().unwrap().push(req.context.clone());
            Ok(req.text.clone())
        }
    }

    struct FailingTranslator;
    impl Translator for FailingTranslator {
        fn name(&self) -> &str {
            "fail"
        }
        fn translate(&mut self, _: &TranslateRequest) -> Result<String, TranslateError> {
            Err(TranslateError::Request("boom".into()))
        }
    }

    fn cfg(tgt: Option<&str>) -> EngineConfig {
        EngineConfig {
            model_path: "unused".into(),
            model_id: "test-model".into(),
            use_gpu: false,
            source_lang: None,
            tgt_lang: tgt.map(str::to_string),
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

    /// FakeSource 로 파이프라인을 돌려 `done` 이 참이 될 때까지 기다린 뒤 멈추고 이벤트를 모은다.
    /// 정지는 대기 중인 번역을 버리므로(I4), 번역을 보려면 `done` 이 그것까지 기다려야 한다.
    fn run(
        cfg: EngineConfig,
        transcriber: Box<dyn Transcriber>,
        translator: Option<Box<dyn Translator>>,
        done: impl Fn(&[EngineEvent]) -> bool,
    ) -> Vec<EngineEvent> {
        let (tx, rx) = mpsc::channel();
        let handle = start(
            cfg,
            Box::new(FakeSource { stop: None }),
            transcriber,
            translator,
            false,
            false,
            tx,
        )
        .unwrap();
        let mut events = Vec::new();
        drain_until(&rx, &mut events, 5, done);
        handle.stop();
        drain_until(&rx, &mut events, 5, |ev| {
            matches!(ev.last(), Some(EngineEvent::Stopped))
        });
        events
    }

    fn count_stopped(events: &[EngineEvent]) -> usize {
        events
            .iter()
            .filter(|e| matches!(e, EngineEvent::Stopped))
            .count()
    }

    /// 번역 큐가 꽉 차도 전사 루프는 멈추지 않는다(C1). 넘치는 조각은 버려져 원문만 남는다.
    #[test]
    fn full_translate_queue_drops_jobs_instead_of_blocking_finals() {
        let (jobs_tx, jobs_rx) = mpsc::sync_channel::<Job>(8);
        // 수신자를 붙들고 하나도 꺼내지 않는다 — 용량 1이 차면 그 뒤는 전부 넘친다.
        let (ttx, trx) = mpsc::sync_channel::<TranslateJob>(1);
        let (tx, rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut t = FakeTranscriber;
            transcribe_loop(jobs_rx, &mut t, None, tx, Some(ttx), true);
        });
        for i in 0..5u64 {
            jobs_tx
                .send(Job {
                    ev: ChunkEvent::Final {
                        pcm: vec![0.0; 16],
                        start_ms: i * 1000,
                        end_ms: i * 1000 + 900,
                    },
                    enqueued: Instant::now(),
                })
                .unwrap();
        }
        drop(jobs_tx);
        worker.join().unwrap();

        let events: Vec<_> = rx.iter().collect();
        let finals = events
            .iter()
            .filter(|e| matches!(e, EngineEvent::Final { .. }))
            .count();
        assert_eq!(finals, 5, "큐가 차도 Final 은 다 나와야 한다: {events:?}");
        let queued = trx.try_iter().count();
        assert!(
            queued < finals,
            "넘친 조각은 버려져야 한다(queued={queued})"
        );
    }

    /// 정지 신호가 켜지면 남은 번역 작업은 번역하지 않고 비운다(I4).
    #[test]
    fn stop_discards_pending_translations_quickly() {
        let (ttx, trx) = mpsc::channel::<TranslateJob>();
        let (tx, rx) = mpsc::channel();
        for i in 1..=8u64 {
            ttx.send((i, format!("s{i}"), "en".into())).unwrap();
        }
        drop(ttx);
        let discard = Arc::new(AtomicBool::new(true));
        let started = Instant::now();
        translate_loop(trx, Box::new(SlowTranslator), "ko".into(), tx, discard);
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "정지가 느리다: {elapsed:?}"
        );

        let events: Vec<_> = rx.iter().collect();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EngineEvent::Translated { .. })),
            "버린 조각은 번역되지 않는다: {events:?}"
        );
        assert_eq!(count_stopped(&events), 1, "Stopped 는 정확히 한 번");
    }

    /// 세 번째 조각은 직전 두 원문을 순서대로 context 로 받는다(§6).
    #[test]
    fn third_job_gets_the_previous_two_finals_as_context() {
        let (ttx, trx) = mpsc::channel::<TranslateJob>();
        let (tx, _rx) = mpsc::channel();
        for (i, text) in ["one", "two", "three"].iter().enumerate() {
            ttx.send((i as u64 + 1, (*text).into(), "en".into()))
                .unwrap();
        }
        drop(ttx);
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        translate_loop(
            trx,
            Box::new(RecordingTranslator(seen.clone())),
            "ko".into(),
            tx,
            Arc::new(AtomicBool::new(false)),
        );
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 3);
        assert!(seen[0].is_empty());
        assert_eq!(seen[1], vec!["one".to_string()]);
        assert_eq!(seen[2], vec!["one".to_string(), "two".to_string()]);
    }

    #[test]
    fn transcriber_panic_emits_error_and_engine_still_stops() {
        let (tx, rx) = mpsc::channel();
        let handle = start(
            cfg(None),
            Box::new(FakeSource { stop: None }),
            Box::new(PanickyTranscriber {
                calls: AtomicUsize::new(0),
            }),
            None,
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
        assert_eq!(count_stopped(&events), 1, "Stopped 는 정확히 한 번");
    }

    #[test]
    fn pipeline_emits_started_final_and_stopped() {
        let events = run(cfg(None), Box::new(FakeTranscriber), None, |ev| {
            ev.iter().any(|e| matches!(e, EngineEvent::Final { .. }))
        });

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
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EngineEvent::Translated { .. })),
            "번역기가 없으면 Translated 가 없어야 한다"
        );
    }

    #[test]
    fn final_is_followed_by_translated_with_same_id_and_stopped_once() {
        let events = run(
            cfg(Some("ko")),
            Box::new(FakeTranscriber),
            Some(Box::new(UpperTranslator)),
            |ev| {
                ev.iter()
                    .any(|e| matches!(e, EngineEvent::Translated { .. }))
            },
        );

        assert!(
            matches!(&events[0], EngineEvent::Started { target_lang, .. } if target_lang.as_deref() == Some("ko")),
            "Started 는 번역 타겟을 실어 나른다: {events:?}"
        );
        let (fid, ftext) = events
            .iter()
            .find_map(|e| match e {
                EngineEvent::Final { id, text, .. } => Some((*id, text.clone())),
                _ => None,
            })
            .expect("a Final event");
        let tr = events
            .iter()
            .find_map(|e| match e {
                EngineEvent::Translated { id, text, lang } if *id == fid => {
                    Some((text.clone(), lang.clone()))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Translated for id {fid}: {events:?}"));
        assert_eq!(tr.0, ftext.to_uppercase());
        assert_eq!(tr.1, "ko");
        assert!(
            matches!(events.last(), Some(EngineEvent::Stopped)),
            "마지막 이벤트는 Stopped 여야 한다: {events:?}"
        );
        assert_eq!(count_stopped(&events), 1, "Stopped 는 정확히 한 번");
    }

    #[test]
    fn no_translation_when_source_equals_target() {
        let events = run(
            cfg(Some("en")),
            Box::new(FakeTranscriber),
            Some(Box::new(UpperTranslator)),
            |ev| ev.iter().any(|e| matches!(e, EngineEvent::Final { .. })),
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, EngineEvent::Final { .. })));
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EngineEvent::Translated { .. })),
            "원어 == 타겟이면 Translated 가 없어야 한다: {events:?}"
        );
        assert!(matches!(events.last(), Some(EngineEvent::Stopped)));
        assert_eq!(count_stopped(&events), 1);
    }

    #[test]
    fn failing_translator_emits_one_translate_error_and_keeps_finals() {
        let events = run(
            cfg(Some("ko")),
            Box::new(FakeTranscriber),
            Some(Box::new(FailingTranslator)),
            |ev| {
                ev.iter()
                    .any(|e| matches!(e, EngineEvent::Error { code, .. } if code == "translate"))
            },
        );
        let finals = events
            .iter()
            .filter(|e| matches!(e, EngineEvent::Final { .. }))
            .count();
        assert!(finals >= 1, "Final 은 그대로 나와야 한다: {events:?}");
        let errors = events
            .iter()
            .filter(|e| matches!(e, EngineEvent::Error { code, .. } if code == "translate"))
            .count();
        assert_eq!(errors, 1, "연속 실패는 한 번만 보고한다: {events:?}");
        assert!(matches!(events.last(), Some(EngineEvent::Stopped)));
        assert_eq!(count_stopped(&events), 1);
    }

    #[test]
    fn lang_vote_majority_overrides_a_single_misdetection() {
        let mut v = LangVote::new();
        assert_eq!(v.push("en".into()), "en", "첫 Final 은 감지값 그대로");
        assert_eq!(v.push("en".into()), "en");
        assert_eq!(v.push("cy".into()), "en", "[en, en, cy] 다수결은 en");
    }

    #[test]
    fn lang_vote_tie_keeps_the_current_detection() {
        let mut v = LangVote::new();
        v.push("en".into());
        assert_eq!(v.push("ko".into()), "ko", "[en, ko] 동률이면 이번 값");
        assert_eq!(v.push("ja".into()), "ja", "[en, ko, ja] 모두 1표면 이번 값");
    }

    #[test]
    fn lang_vote_follows_a_real_switch_after_two_finals() {
        let mut v = LangVote::new();
        v.push("en".into());
        v.push("en".into());
        assert_eq!(v.push("ko".into()), "en", "첫 전환 Final 은 아직 en");
        assert_eq!(v.push("ko".into()), "ko", "[en, ko, ko] 부터 ko");
    }
}
