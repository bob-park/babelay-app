//! 캡처 세션 수명 관리. 엔진 핸들을 들고 있고, 엔진 이벤트를 창으로 중계한다.
use crate::{models::models_dir, settings::SettingsState};
use babelay_engine::engine::{start_default, EngineConfig, EngineEvent, EngineHandle};
use babelay_engine::models::{find, installed, model_path};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Mutex,
};
use std::thread::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};

/// 세션 상태. 모델 로드는 수 초, 정지는 수십 초 걸리므로 그 사이에도 슬롯이
/// 비어 보이면 안 된다(트레이+단축키 동시 입력 → 엔진 2개).
///
/// `Starting` 의 `u64` 는 시작 시도 세대다. 시작 스레드는 자기 세대의
/// 예약이 그대로 남아 있을 때만 핸들을 설치한다 — start→stop→start 가 로드
/// 시간 안에 겹쳐도 남의 `Running` 을 덮어쓰지 않는다.
///
/// 전이:
/// - `start`: Idle → Starting(g) → 내 예약이 남아 있으면 Running(h), 아니면 새 핸들만 세우고 상태는 그대로
/// - 로드 실패: Starting(g) 일 때만 Idle + `engine-event Error{start_failed}`
/// - 로드 중 `stop`: Starting(_) → Idle(시작 스레드가 갓 만든 핸들을 버린다)
/// - `stop`: Running → Stopping(드레인 스레드) → 중계 루프가 `Stopped` 를 보면(아직 Stopping 일 때만) Idle
/// - `stop_on_exit`: Running 은 동기로 세우고, Stopping 이면 드레인 스레드를 join 한다.
///   Starting 이면 Idle 로 두고 나간다(시작 직후 종료 시 핸들 정리가 프로세스 종료와 경합).
#[derive(Default)]
pub enum Phase {
    #[default]
    Idle,
    Starting(u64),
    Running(EngineHandle),
    Stopping(JoinHandle<()>),
}

#[derive(Default)]
pub struct SessionState {
    phase: Mutex<Phase>,
    next_gen: AtomicU64,
    /// 기록 중인 히스토리 세션 행 id(`history`가 읽고 쓴다).
    pub session_id: Mutex<Option<i64>>,
}

fn state(app: &AppHandle) -> &SessionState {
    app.state::<SessionState>().inner()
}

/// 잠금이 오염돼도 캡처 토글이 죽지 않게 한다.
fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, Phase> {
    state(app).phase.lock().unwrap_or_else(|p| p.into_inner())
}

pub fn is_capturing(app: &AppHandle) -> bool {
    matches!(*lock(app), Phase::Starting(_) | Phase::Running(_))
}

/// 모델·경로 검증만 동기로 하고(`unknown_model` / `model_missing`), 실제 로드는
/// 백그라운드로 넘긴다. 호출자(트레이·단축키·커맨드)는 즉시 돌아온다.
pub fn start(app: &AppHandle) -> Result<(), String> {
    let settings = app.state::<SettingsState>().get();
    let m = find(&settings.asr.model_id).ok_or("unknown_model")?;
    let dir = models_dir(app)?;
    if !installed(&dir, m) {
        return Err("model_missing".into());
    }
    let cfg = EngineConfig {
        model_path: model_path(&dir, m),
        use_gpu: settings.asr.gpu,
        source_lang: (settings.asr.source_lang != "auto").then(|| settings.asr.source_lang.clone()),
    };
    let gen = {
        let mut phase = lock(app);
        match *phase {
            Phase::Idle => {}
            // 드레인이 끝나야 다음 캡처를 걸 수 있다. 조용히 삼키지 않는다.
            Phase::Stopping(_) => return Err("busy_stopping".into()),
            _ => return Ok(()), // 이미 시작 중이거나 실행 중
        }
        let gen = state(app).next_gen.fetch_add(1, Ordering::Relaxed);
        *phase = Phase::Starting(gen);
        gen
    };
    crate::tray::relabel_capture(app, true);
    let app2 = app.clone();
    let log = (
        settings.asr.source_lang.clone(),
        settings.overlay.subtitle_lang.clone(),
        settings.asr.model_id.clone(),
    );
    std::thread::spawn(move || run_session(app2, cfg, gen, log));
    Ok(())
}

/// 모델 로드부터 이벤트 중계까지 한 스레드에서 돈다.
fn run_session(app: AppHandle, cfg: EngineConfig, gen: u64, log: (String, String, String)) {
    let (tx, rx) = mpsc::channel();
    let handle = match start_default(cfg, tx) {
        Ok(h) => h,
        Err(message) => {
            {
                let mut phase = lock(&app);
                if matches!(*phase, Phase::Starting(g) if g == gen) {
                    *phase = Phase::Idle;
                }
            }
            let _ = app.emit(
                "engine-event",
                EngineEvent::Error {
                    code: "start_failed".into(),
                    message,
                },
            );
            crate::tray::relabel_capture(&app, is_capturing(&app));
            return;
        }
    };
    {
        let mut phase = lock(&app);
        if !matches!(*phase, Phase::Starting(g) if g == gen) {
            // 내 예약이 사라졌다(stop 또는 더 새로운 start). 갓 만든 엔진만 세우고
            // 상태는 건드리지 않는다 — 남의 Running 을 덮으면 엔진이 새어나간다.
            drop(phase);
            handle.stop();
            crate::tray::relabel_capture(&app, is_capturing(&app));
            return;
        }
        *phase = Phase::Running(handle);
    }
    crate::history::begin(&app, &log.0, &log.1, &log.2);
    // EngineHandle 이 tx 클론을 붙들고 있어 rx 는 스스로 닫히지 않는다. Stopped 에서 끊는다.
    for ev in rx {
        crate::history::on_final(&app, &ev);
        let stopped = matches!(ev, EngineEvent::Stopped);
        let _ = app.emit("engine-event", &ev);
        if stopped {
            crate::history::end(&app);
            break;
        }
    }
    {
        let mut phase = lock(&app);
        if matches!(*phase, Phase::Stopping(_)) {
            *phase = Phase::Idle;
        }
    }
    crate::tray::relabel_capture(&app, is_capturing(&app));
}

/// `EngineHandle::stop()` 은 큐를 다 비울 때까지 수십 초 블로킹할 수 있다.
/// 호출자를 잡지 않도록 드레인은 별도 스레드로 넘기고 라벨은 곧바로 되돌린다.
pub fn stop(app: &AppHandle) {
    let mut phase = lock(app);
    match std::mem::replace(&mut *phase, Phase::Idle) {
        Phase::Running(h) => {
            *phase = Phase::Stopping(std::thread::spawn(move || h.stop()));
            drop(phase);
        }
        // Starting 은 Idle 로 남긴다 — 시작 스레드가 자기 핸들을 알아서 버린다.
        Phase::Starting(_) => drop(phase),
        other => {
            *phase = other;
            return;
        }
    }
    crate::tray::relabel_capture(app, false);
}

/// 종료 경로. 오디오 탭이 살아 있는 채로 프로세스가 죽지 않도록 여기서는 기다린다.
/// 이미 정지 중이면 드레인 스레드를 join 한다 — 대기 시간은 어차피 같은 큐 배수이고,
/// 타임아웃을 두면 탭이 남은 채로 죽을 수 있어 그냥 기다린다.
pub fn stop_on_exit(app: &AppHandle) {
    let mut phase = lock(app);
    match std::mem::replace(&mut *phase, Phase::Idle) {
        Phase::Running(h) => {
            drop(phase);
            h.stop();
        }
        Phase::Stopping(drain) => {
            drop(phase);
            let _ = drain.join();
        }
        _ => return,
    }
    // 중계 루프가 Stopped 를 보기 전에 프로세스가 죽을 수 있다. ended_at 은 여기서 닫는다.
    crate::history::end(app);
}

pub fn toggle(app: &AppHandle) -> Result<(), String> {
    if is_capturing(app) {
        stop(app);
        Ok(())
    } else {
        start(app)
    }
}
