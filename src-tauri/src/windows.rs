use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const MAIN: &str = "main";
pub const ONBOARDING: &str = "onboarding";

pub fn show_main(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(MAIN) {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    let w = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("/".into()))
        .title("Babelay")
        .inner_size(960.0, 640.0)
        .min_inner_size(720.0, 480.0)
        .build()
        .map_err(|e| e.to_string())?;
    // 닫기 = 숨기기. 앱은 트레이에 남는다.
    let handle = w.clone();
    w.on_window_event(move |e| {
        if let WindowEvent::CloseRequested { api, .. } = e {
            api.prevent_close();
            let _ = handle.hide();
        }
    });
    Ok(())
}

pub fn show_onboarding(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(ONBOARDING) {
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(app, ONBOARDING, WebviewUrl::App("/".into()))
        .title("Babelay")
        .inner_size(720.0, 560.0)
        .resizable(false)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn close_onboarding(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(ONBOARDING) {
        let _ = w.close();
    }
}
