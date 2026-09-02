//! 캡처 세션 수명 관리. 엔진 핸들을 들고 있고, 엔진 이벤트를 창으로 중계한다.
use crate::{models::models_dir, settings::SettingsState};
use babelay_engine::engine::{start_default, EngineConfig, EngineEvent, EngineHandle};
use babelay_engine::models::{find, installed, model_path};
use std::sync::{mpsc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

/// 세션 상태. 모델 로드는 수 초, 정지는 수십 초 걸리므로 그 사이에도 슬롯이
/// 비어 보이면 안 된다(트레이+단축키 동시 입력 → 엔진 2개).
///
/// 전이:
/// - `start`: Idle → Starting(잠금 안에서 예약) → 로드 성공이면 Running, 실패면 Idle
/// - 로드 중 `stop` 이 들어오면 Starting → Idle, 시작 스레드가 갓 만든 핸들을 버린다
/// - `stop`: Running → Stopping(드레인은 별도 스레드) → 중계 루프가 `Stopped` 를 보면 Idle
/// - 종료 시 `stop_on_exit` 는 Running 만 동기로 세운다. Starting 이면 Idle 로 두고 나간다
///   (시작 스레드가 핸들을 버리지만, 프로세스가 먼저 죽을 수 있다 — 시작 직후 종료뿐).
#[derive(Default)]
pub enum Phase {
    #[default]
    Idle,
    Starting,
    Running(EngineHandle),
    Stopping,
}

#[derive(Default)]
pub struct SessionState {
    phase: Mutex<Phase>,
}

/// 잠금이 오염돼도 캡처 토글이 죽지 않게 한다.
fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, Phase> {
    app.state::<SessionState>()
        .inner()
        .phase
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

pub fn is_capturing(app: &AppHandle) -> bool {
    matches!(*lock(app), Phase::Starting | Phase::Running(_))
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
    {
        let mut phase = lock(app);
        if !matches!(*phase, Phase::Idle) {
            return Ok(());
        }
        *phase = Phase::Starting;
    }
    crate::tray::relabel_capture(app, true);
    let app2 = app.clone();
    std::thread::spawn(move || run_session(app2, cfg));
    Ok(())
}

/// 모델 로드부터 이벤트 중계까지 한 스레드에서 돈다.
fn run_session(app: AppHandle, cfg: EngineConfig) {
    let (tx, rx) = mpsc::channel();
    let handle = match start_default(cfg, tx) {
        Ok(h) => h,
        Err(message) => {
            *lock(&app) = Phase::Idle;
            let _ = app.emit(
                "engine-event",
                EngineEvent::Error {
                    code: "start_failed".into(),
                    message,
                },
            );
            crate::tray::relabel_capture(&app, false);
            return;
        }
    };
    {
        let mut phase = lock(&app);
        if !matches!(*phase, Phase::Starting) {
            // 로드 중에 stop 이 들어왔다. 갓 만든 엔진을 여기서 세운다(이미 백그라운드).
            *phase = Phase::Idle;
            drop(phase);
            handle.stop();
            crate::tray::relabel_capture(&app, false);
            return;
        }
        *phase = Phase::Running(handle);
    }
    // EngineHandle 이 tx 클론을 붙들고 있어 rx 는 스스로 닫히지 않는다. Stopped 에서 끊는다.
    for ev in rx {
        if let EngineEvent::Final { .. } = &ev {
            crate::history::on_final(&app, &ev);
        }
        let stopped = matches!(ev, EngineEvent::Stopped);
        let _ = app.emit("engine-event", &ev);
        if stopped {
            break;
        }
    }
    {
        let mut phase = lock(&app);
        if matches!(*phase, Phase::Stopping) {
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
            *phase = Phase::Stopping;
            drop(phase);
            std::thread::spawn(move || h.stop());
        }
        // Starting 은 Idle 로 남긴다 — 시작 스레드가 새 핸들을 알아서 버린다.
        Phase::Starting => drop(phase),
        other => {
            *phase = other;
            return;
        }
    }
    crate::tray::relabel_capture(app, false);
}

/// 종료 경로. 오디오 탭이 살아 있는 채로 프로세스가 죽지 않도록 여기서는 기다린다.
pub fn stop_on_exit(app: &AppHandle) {
    let mut phase = lock(app);
    if let Phase::Running(h) = std::mem::replace(&mut *phase, Phase::Idle) {
        drop(phase);
        h.stop();
    }
}

pub fn toggle(app: &AppHandle) -> Result<(), String> {
    if is_capturing(app) {
        stop(app);
        Ok(())
    } else {
        start(app)
    }
}
