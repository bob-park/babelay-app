use crate::{i18n, overlay, settings::SettingsState, windows};
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const SHORTCUT_CAPTURE: &str = "CmdOrCtrl+Shift+S";
pub const SHORTCUT_OVERLAY: &str = "CmdOrCtrl+Shift+O";

pub fn toggle_capture(app: &AppHandle) {
    // 엔진은 2단계. 지금은 프론트가 이 이벤트로 capturing 플래그를 뒤집는다.
    let _ = app.emit("capture-toggle", ());
}

pub fn toggle_overlay(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<SettingsState>();
    let mut s = state.get();
    s.overlay.enabled = !s.overlay.enabled;
    overlay::set_visible(app, s.overlay.enabled)?;
    state.set(app, s)
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let settings = app.state::<SettingsState>().get();
    let labels = i18n::tray_labels(i18n::resolve(&settings.general.ui_language));

    let capture = MenuItem::with_id(app, "capture", labels.start, true, None::<&str>)?;
    let overlay_item = MenuItem::with_id(app, "overlay", labels.overlay_off, true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", labels.open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", labels.quit, true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .items(&[&capture, &overlay_item])
        .separator()
        .items(&[&open])
        .separator()
        .items(&[&quit])
        .build()?;

    #[cfg(target_os = "macos")]
    let icon = Image::from_bytes(include_bytes!("../icons/tray-22.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = Image::from_bytes(include_bytes!("../icons/tray-win-32.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture" => toggle_capture(app),
            "overlay" => {
                let _ = toggle_overlay(app);
            }
            "open" => {
                let _ = windows::show_main(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    let capture_sc: Shortcut = SHORTCUT_CAPTURE.parse().expect("valid shortcut");
    let overlay_sc: Shortcut = SHORTCUT_OVERLAY.parse().expect("valid shortcut");
    app.global_shortcut().on_shortcut(capture_sc, |app, _, ev| {
        if ev.state() == ShortcutState::Pressed {
            toggle_capture(app);
        }
    })
        .map_err(std::io::Error::other)?;
    app.global_shortcut().on_shortcut(overlay_sc, |app, _, ev| {
        if ev.state() == ShortcutState::Pressed {
            let _ = toggle_overlay(app);
        }
    })
        .map_err(std::io::Error::other)?;
    Ok(())
}
