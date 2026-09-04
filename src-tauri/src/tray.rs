use crate::{
    i18n, overlay, session,
    settings::{Settings, SettingsState},
    windows,
};
use babelay_engine::engine::EngineEvent;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Wry,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const SHORTCUT_CAPTURE: &str = "CmdOrCtrl+Shift+S";
pub const SHORTCUT_OVERLAY: &str = "CmdOrCtrl+Shift+O";

pub fn toggle_capture(app: &AppHandle) {
    if let Err(code) = session::toggle(app) {
        // 코드를 그대로 넘겨야 프런트가 busy_stopping / model_missing / unknown_model 을
        // 각각 번역한다. start_failed 는 비동기 로드 실패 경로 전용이다.
        let _ = app.emit(
            "engine-event",
            EngineEvent::Error {
                code,
                message: String::new(),
            },
        );
    }
}

pub fn toggle_overlay(app: &AppHandle) -> Result<(), String> {
    overlay::exit_adjust_mode(app)?; // 조정 모드 중 숨기면 플래그가 남는다
    let state = app.state::<SettingsState>();
    let mut s = state.get();
    s.overlay.enabled = !s.overlay.enabled;
    if s.overlay.enabled {
        overlay::apply_position(app, &s)?;
    }
    overlay::set_visible(app, s.overlay.enabled)?;
    state.set(app, s)
}

/// 트레이 메뉴 항목 핸들. UI 언어가 바뀌면 라벨을 다시 쓴다.
pub struct TrayItems {
    pub capture: MenuItem<Wry>,
    pub overlay: MenuItem<Wry>,
    pub open: MenuItem<Wry>,
    pub quit: MenuItem<Wry>,
}

/// 설정이 저장될 때마다 호출된다. 트레이가 아직 없으면 아무것도 하지 않는다.
pub fn relabel(app: &AppHandle, settings: &Settings) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let l = i18n::tray_labels(i18n::resolve(&settings.general.ui_language));
    // 언어만 바뀐 경우에도 캡처 라벨은 현재 상태를 따라간다.
    let _ = items
        .capture
        .set_text(capture_label(&l, session::is_capturing(app)));
    let _ = items.overlay.set_text(if settings.overlay.enabled {
        l.overlay_off
    } else {
        l.overlay_on
    });
    let _ = items.open.set_text(l.open);
    let _ = items.quit.set_text(l.quit);
}

/// 캡처 시작/정지 시 트레이 라벨만 갱신한다. 트레이가 아직 없으면 무시된다.
pub fn relabel_capture(app: &AppHandle, capturing: bool) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let lang = i18n::resolve(&app.state::<SettingsState>().get().general.ui_language);
    let _ = items
        .capture
        .set_text(capture_label(&i18n::tray_labels(lang), capturing));
}

fn capture_label(l: &i18n::TrayLabels, capturing: bool) -> &'static str {
    if capturing {
        l.stop
    } else {
        l.start
    }
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let settings = app.state::<SettingsState>().get();
    let labels = i18n::tray_labels(i18n::resolve(&settings.general.ui_language));

    let capture = MenuItem::with_id(app, "capture", labels.start, true, None::<&str>)?;
    let overlay_label = if settings.overlay.enabled {
        labels.overlay_off
    } else {
        labels.overlay_on
    };
    let overlay_item = MenuItem::with_id(app, "overlay", overlay_label, true, None::<&str>)?;
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
        // Windows 관습: 트레이 아이콘 더블클릭 = 창 열기. macOS 는 이 이벤트를 내지 않는다.
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                let _ = windows::show_main(tray.app_handle());
            }
        })
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

    app.manage(TrayItems {
        capture,
        overlay: overlay_item,
        open,
        quit,
    });

    let capture_sc: Shortcut = SHORTCUT_CAPTURE.parse().expect("valid shortcut");
    let overlay_sc: Shortcut = SHORTCUT_OVERLAY.parse().expect("valid shortcut");
    // Another app may already own the hotkey (Windows rejects duplicates);
    // the tray menu covers the same actions, so warn instead of failing setup.
    if let Err(e) = app.global_shortcut().on_shortcut(capture_sc, |app, _, ev| {
        if ev.state() == ShortcutState::Pressed {
            toggle_capture(app);
        }
    }) {
        eprintln!("shortcut {SHORTCUT_CAPTURE} unavailable: {e}");
    }
    if let Err(e) = app.global_shortcut().on_shortcut(overlay_sc, |app, _, ev| {
        if ev.state() == ShortcutState::Pressed {
            let _ = toggle_overlay(app);
        }
    }) {
        eprintln!("shortcut {SHORTCUT_OVERLAY} unavailable: {e}");
    }
    Ok(())
}
