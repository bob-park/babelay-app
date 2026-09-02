use crate::{
    overlay,
    settings::{Settings, SettingsState},
    windows,
};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    state.get()
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    state: State<'_, SettingsState>,
    settings: Settings,
) -> Result<(), String> {
    let before = state.get();
    state.set(&app, settings.clone())?;
    if before.overlay != settings.overlay {
        overlay::apply_position(&app, &settings)?;
        overlay::set_visible(&app, settings.overlay.enabled)?;
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
    if cfg!(target_os = "windows") { "granted".into() } else { "unknown".into() }
}

#[tauri::command]
pub fn open_privacy_settings(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let url = if cfg!(target_os = "macos") {
        "x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture"
    } else {
        "ms-settings:privacy-microphone"
    };
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
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
    overlay::set_adjust_mode(&app, enabled)?;
    if !enabled {
        let s = app.state::<SettingsState>().get();
        overlay::set_visible(&app, s.overlay.enabled)?;
    }
    Ok(())
}

#[tauri::command]
pub fn overlay_get_monitors(app: AppHandle) -> Result<Vec<overlay::MonitorInfo>, String> {
    overlay::monitors(&app)
}

#[tauri::command]
pub fn overlay_commit_position(app: AppHandle) -> Result<(), String> {
    overlay::commit_position(&app)
}
