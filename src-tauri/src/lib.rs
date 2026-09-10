mod commands;
mod history;
mod i18n;
mod keys;
mod llm;
mod models;
mod overlay;
mod session;
mod settings;
mod translator;
mod tray;
mod updater;
mod windows;

use session::SessionState;
use settings::SettingsState;
use tauri::AppHandle;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            app.manage(SettingsState::new(path));
            app.manage(models::Downloads::default());
            app.manage(llm::LlmCache::new(app.handle().clone()));
            app.manage(SessionState::default());
            app.manage(updater::UpdateState::default());
            // 기록은 있으면 좋은 기능이다. 열지 못해도 앱은 떠야 한다.
            match history::open(&app.path().app_local_data_dir()?.join("history.sqlite")) {
                Ok(db) => {
                    app.manage(db);
                }
                Err(e) => eprintln!("babelay history: disabled: {e}"),
            }
            babelay_engine::transcribe::install_logging_hooks();
            let settings = app.state::<SettingsState>().get();
            let handle = app.handle().clone();
            if settings.general.onboarding_done {
                windows::show_main(&handle).map_err(std::io::Error::other)?;
            } else {
                windows::show_onboarding(&handle).map_err(std::io::Error::other)?;
            }
            overlay::create(&handle, &settings).map_err(std::io::Error::other)?;
            tray::build(&handle)?;
            updater::spawn_periodic(handle.clone());
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
            commands::overlay_commit_position,
            commands::get_models,
            commands::get_presets,
            commands::get_hw_info,
            commands::start_capture,
            commands::stop_capture,
            commands::capture_state,
            commands::download_model,
            commands::cancel_download,
            commands::delete_model,
            commands::history_sessions,
            commands::history_segments,
            commands::history_search,
            commands::history_delete,
            commands::history_export,
            commands::set_api_key,
            commands::has_api_key,
            commands::delete_api_key,
            commands::test_translation,
            commands::update_status,
            commands::check_update,
            commands::install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                shutdown(app);
            }
        });
}

/// 종료·재시작 전에 엔진을 세운다. 오디오 탭이 살아 있는 채로 프로세스가 죽으면 안 되고,
/// 번역 모델이 Metal 버퍼를 쥔 채 exit() 가 돌면 ggml 정적 소멸자가 abort 한다.
pub fn shutdown(app: &AppHandle) {
    session::stop_on_exit(app);
    llm::cache(app).clear();
}
