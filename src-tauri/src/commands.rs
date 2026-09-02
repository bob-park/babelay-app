use crate::{
    overlay,
    settings::{self, Settings, SettingsState},
    windows,
};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

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

// ponytail: 2단계에서 Core Audio 탭 생성 시도로 교체한다.
#[tauri::command]
pub fn check_audio_permission() -> String {
    if cfg!(target_os = "windows") {
        "granted".into()
    } else {
        "unknown".into()
    }
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
pub fn get_hw_info() -> babelay_engine::hardware::HwInfo {
    crate::models::hw().clone()
}
