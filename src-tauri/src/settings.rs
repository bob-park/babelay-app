use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter};

// ponytail: 열거형 대신 String. 프론트 TS 유니온이 값을 제한하고,
// 모르는 값은 각 소비처에서 기본값으로 취급한다.

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub general: General,
    pub asr: Asr,
    pub translation: Translation,
    pub overlay: Overlay,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct General {
    pub theme: String,       // system | dark | light
    pub ui_language: String, // system | ko | en | ja
    pub onboarding_done: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Asr {
    pub model_id: String,
    pub gpu: bool,
    pub source_lang: String, // auto | ko | en | ja
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Translation {
    pub backend: String, // local | cloud
    pub local_model: String,
    pub cloud: Cloud,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Cloud {
    pub provider: String, // openai | anthropic | gemini | deepl | custom
    pub model: String,
    pub base_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Overlay {
    pub enabled: bool,
    pub monitor_id: String, // "" = 주 모니터
    pub x_ratio: f64,
    pub y_ratio: f64,
    pub w_ratio: f64,
    pub display_mode: String, // both | source | target
    pub subtitle_lang: String, // system | ko | en | ja
    pub font_size: u32,
    pub bg_opacity: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            general: General::default(),
            asr: Asr::default(),
            translation: Translation::default(),
            overlay: Overlay::default(),
        }
    }
}

impl Default for General {
    fn default() -> Self {
        Self { theme: "system".into(), ui_language: "system".into(), onboarding_done: false }
    }
}

impl Default for Asr {
    fn default() -> Self {
        Self { model_id: "small".into(), gpu: true, source_lang: "auto".into() }
    }
}

impl Default for Translation {
    fn default() -> Self {
        Self { backend: "local".into(), local_model: "qwen3.5-2b".into(), cloud: Cloud::default() }
    }
}

impl Default for Cloud {
    fn default() -> Self {
        Self { provider: "openai".into(), model: "gpt-4o-mini".into(), base_url: String::new() }
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            enabled: true,
            monitor_id: String::new(),
            x_ratio: 0.5,
            y_ratio: 0.85,
            w_ratio: 0.6,
            display_mode: "both".into(),
            subtitle_lang: "system".into(),
            font_size: 24,
            bg_opacity: 0.8,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("settings: parse error, using defaults: {e}");
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(&tmp, text)?;
        fs::rename(tmp, path)
    }
}

pub struct SettingsState {
    pub path: PathBuf,
    pub current: Mutex<Settings>,
}

impl SettingsState {
    pub fn new(path: PathBuf) -> Self {
        let current = Mutex::new(Settings::load(&path));
        Self { path, current }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    pub fn set(&self, app: &AppHandle, new: Settings) -> Result<(), String> {
        new.save(&self.path).map_err(|e| e.to_string())?;
        *self.current.lock().unwrap() = new.clone();
        app.emit("settings-changed", &new).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutated_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut s = Settings::default();
        s.general.theme = "dark".into();
        s.asr.gpu = false;
        s.translation.cloud.base_url = "x".into();
        s.overlay.font_size = 33;
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);

        // 필드가 스네이크 케이스 키로 실제 기록되는지 확인 (프론트 TS와 계약).
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"ui_language\""), "{text}");
        assert!(text.contains("\"x_ratio\""), "{text}");
        assert!(text.contains("\"base_url\""), "{text}");
    }

    #[test]
    fn defaults_match_spec() {
        let s = Settings::default();
        assert_eq!(s.version, 1);

        assert_eq!(s.general.theme, "system");
        assert_eq!(s.general.ui_language, "system");
        assert!(!s.general.onboarding_done);

        assert_eq!(s.asr.model_id, "small");
        assert!(s.asr.gpu);
        assert_eq!(s.asr.source_lang, "auto");

        assert_eq!(s.translation.backend, "local");
        assert_eq!(s.translation.local_model, "qwen3.5-2b");
        assert_eq!(s.translation.cloud.provider, "openai");
        assert_eq!(s.translation.cloud.model, "gpt-4o-mini");
        assert_eq!(s.translation.cloud.base_url, "");

        assert!(s.overlay.enabled);
        assert_eq!(s.overlay.monitor_id, "");
        assert_eq!(s.overlay.x_ratio, 0.5);
        assert_eq!(s.overlay.y_ratio, 0.85);
        assert_eq!(s.overlay.w_ratio, 0.6);
        assert_eq!(s.overlay.display_mode, "both");
        assert_eq!(s.overlay.subtitle_lang, "system");
        assert_eq!(s.overlay.font_size, 24);
        assert_eq!(s.overlay.bg_opacity, 0.8);
    }

    #[test]
    fn missing_fields_are_filled_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"version":1,"general":{"theme":"dark"}}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.general.theme, "dark");
        assert_eq!(s.general.ui_language, "system");
        assert_eq!(s.overlay.font_size, 24);
    }

    #[test]
    fn corrupt_file_falls_back_to_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn missing_file_is_default() {
        assert_eq!(Settings::load(Path::new("/nonexistent/settings.json")), Settings::default());
    }
}
