#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ko,
    En,
    Ja,
}

pub fn resolve(pref: &str) -> Lang {
    let system = sys_locale::get_locale();
    resolve_with(pref, system.as_deref())
}

pub fn resolve_with(pref: &str, system: Option<&str>) -> Lang {
    let code = if pref == "system" {
        system.unwrap_or("en")
    } else {
        pref
    };
    let primary = code
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match primary.as_str() {
        "ko" => Lang::Ko,
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}

pub struct TrayLabels {
    pub start: &'static str,
    /// 캡처 상태 토글은 2단계.
    #[allow(dead_code)]
    pub stop: &'static str,
    pub overlay_on: &'static str,
    pub overlay_off: &'static str,
    pub open: &'static str,
    pub quit: &'static str,
}

pub fn tray_labels(lang: Lang) -> TrayLabels {
    match lang {
        Lang::Ko => TrayLabels {
            start: "캡처 시작",
            stop: "캡처 정지",
            overlay_on: "오버레이 켜기",
            overlay_off: "오버레이 끄기",
            open: "Babelay 열기",
            quit: "종료",
        },
        Lang::En => TrayLabels {
            start: "Start Capture",
            stop: "Stop Capture",
            overlay_on: "Show Overlay",
            overlay_off: "Hide Overlay",
            open: "Open Babelay",
            quit: "Quit",
        },
        Lang::Ja => TrayLabels {
            start: "キャプチャ開始",
            stop: "キャプチャ停止",
            overlay_on: "オーバーレイを表示",
            overlay_off: "オーバーレイを非表示",
            open: "Babelay を開く",
            quit: "終了",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_pref_wins() {
        assert_eq!(resolve_with("ja", Some("ko-KR")), Lang::Ja);
    }

    #[test]
    fn system_uses_locale_prefix() {
        assert_eq!(resolve_with("system", Some("ko-KR")), Lang::Ko);
        assert_eq!(resolve_with("system", Some("ja")), Lang::Ja);
        assert_eq!(resolve_with("system", Some("en-US")), Lang::En);
    }

    #[test]
    fn unsupported_falls_back_to_english() {
        assert_eq!(resolve_with("system", Some("de-DE")), Lang::En);
        assert_eq!(resolve_with("system", None), Lang::En);
        assert_eq!(resolve_with("zz", Some("ko")), Lang::En);
    }

    #[test]
    fn tray_labels_are_localized() {
        assert_eq!(tray_labels(Lang::Ko).quit, "종료");
        assert_eq!(tray_labels(Lang::En).quit, "Quit");
        assert_eq!(tray_labels(Lang::Ja).quit, "終了");
    }
}
