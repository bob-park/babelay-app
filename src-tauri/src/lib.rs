mod commands;
mod i18n;
mod overlay;
mod settings;
mod tray;
mod windows;

use settings::SettingsState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            app.manage(SettingsState::new(path));
            let settings = app.state::<SettingsState>().get();
            let handle = app.handle().clone();
            if settings.general.onboarding_done {
                windows::show_main(&handle).map_err(std::io::Error::other)?;
            } else {
                windows::show_onboarding(&handle).map_err(std::io::Error::other)?;
            }
            overlay::create(&handle, &settings).map_err(std::io::Error::other)?;
            tray::build(&handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::patch_settings,
            commands::get_platform,
            commands::check_audio_permission,
            commands::open_privacy_settings,
            commands::finish_onboarding,
            commands::overlay_set_adjust_mode,
            commands::overlay_get_monitors,
            commands::overlay_commit_position,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
