use crate::settings::{Overlay, Settings, SettingsState};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};

pub const LABEL: &str = "overlay";
pub static ADJUST_MODE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct MonitorInfo {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}

/// 비율 → 물리 좌표. 높이는 모니터의 20%로 고정한다.
pub fn rect_from(o: &Overlay, mon: &Rect) -> Rect {
    let w = (mon.w as f64 * o.w_ratio.clamp(0.2, 1.0)).round() as u32;
    let h = (mon.h as f64 * 0.2).round() as u32;
    let cx = mon.x as f64 + mon.w as f64 * o.x_ratio.clamp(0.0, 1.0);
    let cy = mon.y as f64 + mon.h as f64 * o.y_ratio.clamp(0.0, 1.0);
    Rect {
        x: (cx - w as f64 / 2.0).round() as i32,
        y: (cy - h as f64 / 2.0).round() as i32,
        w,
        h,
    }
}

/// 물리 좌표 → (x_ratio, y_ratio, w_ratio). 창 중심을 기준으로 한다.
pub fn ratios_from(win: &Rect, mon: &Rect) -> (f64, f64, f64) {
    let cx = win.x as f64 + win.w as f64 / 2.0;
    let cy = win.y as f64 + win.h as f64 / 2.0;
    (
        ((cx - mon.x as f64) / mon.w as f64).clamp(0.0, 1.0),
        ((cy - mon.y as f64) / mon.h as f64).clamp(0.0, 1.0),
        (win.w as f64 / mon.w as f64).clamp(0.2, 1.0),
    )
}

fn monitor_id(m: &tauri::Monitor) -> String {
    m.name()
        .cloned()
        .unwrap_or_else(|| format!("{},{}", m.position().x, m.position().y))
}

fn monitor_rect(m: &tauri::Monitor) -> Rect {
    Rect {
        x: m.position().x,
        y: m.position().y,
        w: m.size().width,
        h: m.size().height,
    }
}

pub fn monitors(app: &AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let primary = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .map(|m| monitor_id(&m));
    let list = app.available_monitors().map_err(|e| e.to_string())?;
    Ok(list
        .iter()
        .map(|m| {
            let id = monitor_id(m);
            MonitorInfo {
                primary: Some(&id) == primary.as_ref(),
                id,
                x: m.position().x,
                y: m.position().y,
                width: m.size().width,
                height: m.size().height,
                scale: m.scale_factor(),
            }
        })
        .collect())
}

/// 설정의 monitor_id에 해당하는 모니터. 없으면 주 모니터.
fn target_monitor(app: &AppHandle, id: &str) -> Result<Rect, String> {
    let list = app.available_monitors().map_err(|e| e.to_string())?;
    if let Some(m) = list.iter().find(|m| monitor_id(m) == id) {
        return Ok(monitor_rect(m));
    }
    let primary = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| list.into_iter().next())
        .ok_or("no monitor")?;
    Ok(monitor_rect(&primary))
}

pub fn create(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    if app.get_webview_window(LABEL).is_some() {
        return Ok(());
    }
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("/".into()))
        .title("Babelay Overlay")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    win.set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;
    apply_position(app, settings)?;
    if settings.overlay.enabled {
        win.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn apply_position(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    if ADJUST_MODE.load(Ordering::Relaxed) {
        return Ok(()); // 드래그 중에는 설정 반영으로 창을 되돌리지 않는다
    }
    let Some(win) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    let mon = target_monitor(app, &settings.overlay.monitor_id)?;
    let r = rect_from(&settings.overlay, &mon);
    win.set_size(PhysicalSize::new(r.w, r.h))
        .map_err(|e| e.to_string())?;
    win.set_position(PhysicalPosition::new(r.x, r.y))
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_visible(app: &AppHandle, visible: bool) -> Result<(), String> {
    let Some(win) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    if visible { win.show() } else { win.hide() }.map_err(|e| e.to_string())
}

pub fn set_adjust_mode(app: &AppHandle, enabled: bool) -> Result<(), String> {
    // 창이 없으면 플래그를 세우지 않는다. 세워두면 지울 방법이 없다.
    let Some(win) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    ADJUST_MODE.store(enabled, Ordering::Relaxed);
    win.set_ignore_cursor_events(!enabled)
        .map_err(|e| e.to_string())?;
    if enabled {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    }
    app.emit_to(LABEL, "overlay-adjust-mode", enabled)
        .map_err(|e| e.to_string())
}

/// 현재 창 위치·크기를 비율로 환산해 설정에 저장한다.
pub fn commit_position(app: &AppHandle) -> Result<(), String> {
    let Some(win) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    let pos = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.inner_size().map_err(|e| e.to_string())?;
    let mon = win
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("no monitor")?;
    let (x, y, w) = ratios_from(
        &Rect {
            x: pos.x,
            y: pos.y,
            w: size.width,
            h: size.height,
        },
        &monitor_rect(&mon),
    );
    let state = app.state::<SettingsState>();
    let mut s = state.get();
    s.overlay.monitor_id = monitor_id(&mon);
    s.overlay.x_ratio = x;
    s.overlay.y_ratio = y;
    s.overlay.w_ratio = w;
    state.set(app, s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Overlay;

    fn mon() -> Rect {
        Rect {
            x: 0,
            y: 0,
            w: 2000,
            h: 1000,
        }
    }

    #[test]
    fn default_ratios_give_bottom_center_rect() {
        let r = rect_from(&Overlay::default(), &mon());
        assert_eq!(r.w, 1200); // 0.6 * 2000
        assert_eq!(r.h, 200); // 0.2 * 1000
        assert_eq!(r.x, 400); // center 1000 - 600
        assert_eq!(r.y, 750); // center 850 - 100
    }

    #[test]
    fn ratios_roundtrip() {
        let o = Overlay {
            x_ratio: 0.3,
            y_ratio: 0.2,
            w_ratio: 0.5,
            ..Overlay::default()
        };
        let r = rect_from(&o, &mon());
        let (x, y, w) = ratios_from(&r, &mon());
        assert!((x - 0.3).abs() < 1e-6);
        assert!((y - 0.2).abs() < 1e-6);
        assert!((w - 0.5).abs() < 1e-6);
    }

    #[test]
    fn ratios_are_clamped() {
        let far = Rect {
            x: -5000,
            y: 9000,
            w: 10,
            h: 10,
        };
        let (x, y, w) = ratios_from(&far, &mon());
        assert_eq!((x, y, w), (0.0, 1.0, 0.2));
    }

    #[test]
    fn secondary_monitor_offset_is_respected() {
        let m = Rect {
            x: 2000,
            y: -500,
            w: 1000,
            h: 1000,
        };
        let r = rect_from(&Overlay::default(), &m);
        assert_eq!(r.x, 2000 + 500 - 300);
        assert_eq!(r.y, -500 + 850 - 100);
    }
}
