use crate::{
    overlay,
    settings::{self, Settings, SettingsState},
    windows,
};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    state.get()
}

/// 부분 갱신. 전체 문서를 쓰면 동시 쓰기가 서로를 덮는다.
#[tauri::command]
pub fn patch_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    patch: serde_json::Value,
) -> Result<(), String> {
    let before = state.get();
    let mut merged = serde_json::to_value(&before).map_err(|e| e.to_string())?;
    settings::merge(&mut merged, &patch);
    let next: Settings = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    state.set(&app, next.clone())?;
    // 조정 모드 중에는 창을 건드리지 않는다. 종료할 때 exit_adjust_mode가 다시 맞춘다.
    if before.overlay != next.overlay && !overlay::ADJUST_MODE.load(Ordering::Relaxed) {
        overlay::apply_position(&app, &next)?;
        overlay::set_visible(&app, next.overlay.enabled)?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
pub fn check_audio_permission() -> String {
    use babelay_engine::capture::Permission;
    match babelay_engine::capture::probe_permission() {
        Permission::Granted => "granted",
        Permission::Denied => "denied",
        Permission::Unknown => "unknown",
    }
    .to_string()
}

#[tauri::command]
pub fn start_capture(app: AppHandle) -> Result<(), String> {
    crate::session::start(&app)
}

#[tauri::command]
pub fn stop_capture(app: AppHandle) {
    crate::session::stop(&app)
}

#[tauri::command]
pub fn capture_state(app: AppHandle) -> bool {
    crate::session::is_capturing(&app)
}

#[tauri::command]
pub fn open_privacy_settings(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let url = if cfg!(target_os = "macos") {
        "x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture"
    } else {
        "ms-settings:privacy-microphone"
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn finish_onboarding(app: AppHandle, state: State<'_, SettingsState>) -> Result<(), String> {
    let mut s = state.get();
    s.general.onboarding_done = true;
    state.set(&app, s)?;
    windows::show_main(&app)?;
    windows::close_onboarding(&app);
    Ok(())
}

#[tauri::command]
pub fn overlay_set_adjust_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    // false는 set_adjust_mode 안에서 exit_adjust_mode로 위임된다(유일한 종료 경로).
    overlay::set_adjust_mode(&app, enabled)
}

#[tauri::command]
pub fn overlay_commit_position(app: AppHandle) -> Result<(), String> {
    overlay::commit_position(&app)
}

#[tauri::command]
pub fn get_models(app: AppHandle) -> Result<Vec<crate::models::ModelStatus>, String> {
    crate::models::list(&app)
}

#[tauri::command]
pub fn download_model(app: AppHandle, id: String) -> Result<(), String> {
    crate::models::start(&app, &id)
}

#[tauri::command]
pub fn cancel_download(app: AppHandle, id: String) -> Result<(), String> {
    crate::models::cancel(&app, &id)
}

#[tauri::command]
pub fn delete_model(app: AppHandle, id: String) -> Result<(), String> {
    crate::models::delete(&app, &id)
}

#[tauri::command]
pub fn history_sessions(
    db: State<'_, crate::history::Db>,
    limit: u32,
) -> Result<Vec<crate::history::SessionSummary>, String> {
    db.sessions(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_segments(
    db: State<'_, crate::history::Db>,
    session_id: i64,
) -> Result<Vec<crate::history::SegmentRow>, String> {
    db.segments(session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_search(
    db: State<'_, crate::history::Db>,
    q: String,
) -> Result<Vec<crate::history::SegmentRow>, String> {
    db.search(&q).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_delete(db: State<'_, crate::history::Db>, session_id: i64) -> Result<(), String> {
    db.delete_session(session_id).map_err(|e| e.to_string())
}

/// 다운로드 폴더에 쓰고 저장 경로를 돌려준다.
#[tauri::command]
pub fn history_export(
    app: AppHandle,
    db: State<'_, crate::history::Db>,
    session_id: i64,
    format: String,
) -> Result<String, String> {
    let ext = if format == "srt" { "srt" } else { "txt" };
    let body = db.export(session_id, ext).map_err(|e| e.to_string())?;
    let path = app
        .path()
        .download_dir()
        .map_err(|e| e.to_string())?
        .join(format!("babelay-{session_id}.{ext}"));
    std::fs::write(&path, body).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn get_hw_info() -> babelay_engine::hardware::HwInfo {
    crate::models::hw().clone()
}

#[tauri::command]
pub fn set_api_key(provider: String, key: String) -> Result<(), String> {
    crate::keys::set(&provider, &key)
}

#[tauri::command]
pub fn has_api_key(provider: String) -> bool {
    crate::keys::has(&provider)
}

#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    crate::keys::delete(&provider)
}

/// 연결 테스트 상한. 로컬 모델은 첫 로드가 이 안에 끝나야 한다.
const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// 설정 그대로 한 문장을 번역해 본다. 로컬 LLM 로드가 수 초 걸리므로 워커 스레드에서 돌리고,
/// 기다리는 쪽도 `spawn_blocking` 으로 보내 런타임 워커를 잡지 않는다. 상한을 넘기면 워커는
/// 버린다(끝나면 스스로 사라진다).
#[tauri::command]
pub async fn test_translation(app: AppHandle) -> Result<crate::translator::TestResult, String> {
    let settings = app.state::<SettingsState>().get();
    let dir = crate::models::models_dir(&app)?;
    let cache = crate::llm::cache(&app);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(crate::translator::test_translation(&settings, &dir, &cache));
    });
    tauri::async_runtime::spawn_blocking(move || {
        rx.recv_timeout(TEST_TIMEOUT)
            .map_err(|_| "timeout".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
