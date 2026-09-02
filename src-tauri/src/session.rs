//! 캡처 세션 수명 관리. 엔진 핸들을 들고 있고, 엔진 이벤트를 창으로 중계한다.
use crate::{models::models_dir, settings::SettingsState};
use babelay_engine::engine::{start_default, EngineConfig, EngineEvent, EngineHandle};
use babelay_engine::models::{find, installed, model_path};
use std::sync::{mpsc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Default)]
pub struct SessionState {
    handle: Mutex<Option<EngineHandle>>,
}

/// 잠금이 오염돼도 캡처 토글이 죽지 않게 한다. 슬롯은 매번 통째로 교체된다.
fn lock(s: &SessionState) -> std::sync::MutexGuard<'_, Option<EngineHandle>> {
    s.handle.lock().unwrap_or_else(|p| p.into_inner())
}

fn take(app: &AppHandle) -> Option<EngineHandle> {
    lock(&app.state::<SessionState>()).take()
}

pub fn is_capturing(app: &AppHandle) -> bool {
    lock(&app.state::<SessionState>()).is_some()
}

pub fn start(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<SessionState>();
    if lock(&state).is_some() {
        return Ok(());
    }
    let settings = app.state::<SettingsState>().get();
    let m = find(&settings.asr.model_id).ok_or("unknown model")?;
    let dir = models_dir(app)?;
    if !installed(&dir, m) {
        return Err("model_missing".into());
    }
    let cfg = EngineConfig {
        model_path: model_path(&dir, m),
        use_gpu: settings.asr.gpu,
        source_lang: (settings.asr.source_lang != "auto").then(|| settings.asr.source_lang.clone()),
    };
    let (tx, rx) = mpsc::channel();
    let handle = start_default(cfg, tx)?;
    *lock(&state) = Some(handle);
    let app2 = app.clone();
    // EngineHandle 이 tx 클론을 붙들고 있으므로 rx 는 닫히지 않는다. Stopped 에서 끊는다.
    std::thread::spawn(move || {
        for ev in rx {
            if let EngineEvent::Final { .. } = &ev {
                crate::history::on_final(&app2, &ev);
            }
            let stopped = matches!(ev, EngineEvent::Stopped);
            let _ = app2.emit("engine-event", &ev);
            if stopped {
                break;
            }
        }
        crate::tray::relabel_capture(&app2, is_capturing(&app2));
    });
    crate::tray::relabel_capture(app, true);
    Ok(())
}

/// `EngineHandle::stop()` 은 큐를 다 비울 때까지 수십 초 블로킹할 수 있다.
/// 호출자(트레이·단축키·커맨드)를 잡지 않도록 별도 스레드로 넘긴다.
pub fn stop(app: &AppHandle) {
    if let Some(h) = take(app) {
        std::thread::spawn(move || h.stop());
    }
}

/// 종료 경로. 오디오 탭이 살아 있는 채로 프로세스가 죽지 않도록 여기서는 기다린다.
pub fn stop_on_exit(app: &AppHandle) {
    if let Some(h) = take(app) {
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
