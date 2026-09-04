//! 설정 → 번역기 조립. 조립은 네트워크도 디스크도 타지 않는다 — 로컬 LLM 은 첫 번역에서
//! 로드되고(`llm::SharedLlm`), 동기 시작 경로는 `precheck` 로 키·모델 존재만 확인한다.
use crate::i18n::{resolve, Lang};
use crate::llm::{LlmCache, SharedLlm};
use crate::{keys, settings::Settings};
use babelay_engine::models::{find, installed, model_path};
use babelay_engine::translate::cloud::{Anthropic, DeepL, Gemini, OpenAiCompatible};
use babelay_engine::translate::{TranslateRequest, Translator};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn code(l: Lang) -> &'static str {
    match l {
        Lang::Ko => "ko",
        Lang::En => "en",
        Lang::Ja => "ja",
    }
}

/// 자막 언어 설정을 구체 코드로. `system` 은 OS 로케일을 따른다.
pub fn resolve_tgt(settings: &Settings) -> String {
    match settings.overlay.subtitle_lang.as_str() {
        "system" => code(resolve("system")).into(),
        other => other.into(),
    }
}

/// 표시 모드가 원문만이면 번역하지 않는다.
pub fn enabled(settings: &Settings) -> bool {
    settings.overlay.display_mode != "source"
}

/// 엔진에 넘길 번역 타겟. 번역이 꺼져 있거나, 원어가 고정돼 있고 타겟과 같으면 `None` —
/// 번역 단계를 만들지 않으므로 `Started.target_lang` 도 null 이고 오버레이가 기다리지 않는다.
pub fn target(settings: &Settings) -> Option<String> {
    if !enabled(settings) {
        return None;
    }
    let tgt = resolve_tgt(settings);
    (settings.asr.source_lang != tgt).then_some(tgt)
}

/// 히스토리 `translator` 컬럼 값. `local:<model>` / `cloud:<provider>/<model>`.
/// 번역 단계가 없는 세션(`target` 이 `None`)은 모델을 한 번도 안 쓰므로 `None` 을 기록한다.
pub fn label(settings: &Settings) -> Option<String> {
    target(settings)?;
    let t = &settings.translation;
    Some(if t.backend == "cloud" {
        format!("cloud:{}/{}", t.cloud.provider, t.cloud.model)
    } else {
        format!("local:{}", t.local_model)
    })
}

fn local_model_path(settings: &Settings, models_dir: &Path) -> Result<PathBuf, String> {
    let m = find(&settings.translation.local_model).ok_or("translation_model_missing")?;
    if !installed(models_dir, m) {
        return Err("translation_model_missing".into());
    }
    Ok(model_path(models_dir, m))
}

/// 어댑터 생성은 네트워크를 타지 않는다(키체인 읽기만).
fn cloud_translator(settings: &Settings) -> Result<Box<dyn Translator>, String> {
    let c = &settings.translation.cloud;
    let api_key = keys::get(&c.provider)?.ok_or("api_key_missing")?;
    let model = |default: &str| {
        if c.model.trim().is_empty() {
            default.to_string()
        } else {
            c.model.clone()
        }
    };
    let t: Box<dyn Translator> = match c.provider.as_str() {
        "openai" => Box::new(OpenAiCompatible {
            api_key,
            model: model("gpt-4o-mini"),
            ..Default::default()
        }),
        "custom" => {
            if c.base_url.trim().is_empty() {
                return Err("base_url_missing".into());
            }
            Box::new(OpenAiCompatible {
                base_url: c.base_url.trim().to_string(),
                api_key,
                model: model("gpt-4o-mini"),
            })
        }
        "anthropic" => Box::new(Anthropic {
            api_key,
            model: model("claude-haiku-4-5-20251001"),
            ..Default::default()
        }),
        "gemini" => Box::new(Gemini {
            api_key,
            model: model("gemini-2.5-flash"),
            ..Default::default()
        }),
        "deepl" => Box::new(DeepL::new(api_key)),
        _ => return Err("unknown_provider".into()),
    };
    Ok(t)
}

/// 시작 전 동기 검사: 키/모델 존재만. 로드는 하지 않는다.
pub fn precheck(settings: &Settings, models_dir: &Path) -> Result<(), String> {
    if !enabled(settings) {
        return Ok(());
    }
    if settings.translation.backend == "cloud" {
        cloud_translator(settings).map(|_| ())
    } else {
        local_model_path(settings, models_dir).map(|_| ())
    }
}

/// `None` = 번역 없음(표시 모드 원문만). 로드는 하지 않으므로 즉시 돌아온다.
pub fn build(
    settings: &Settings,
    models_dir: &Path,
    cache: &LlmCache,
) -> Result<Option<Box<dyn Translator>>, String> {
    if !enabled(settings) {
        return Ok(None);
    }
    if settings.translation.backend == "cloud" {
        return Ok(Some(cloud_translator(settings)?));
    }
    let path = local_model_path(settings, models_dir)?;
    Ok(Some(Box::new(SharedLlm::new(
        cache.clone(),
        path,
        settings.asr.gpu,
    ))))
}

#[derive(serde::Serialize, Debug)]
pub struct TestResult {
    pub ok: bool,
    pub ms: u64,
    /// 성공이면 번역 결과, 실패면 상세 메시지.
    pub text: String,
    /// 실패 코드(UI 가 로컬라이즈): `translation_model_missing` 등, 번역 자체 실패는 `translate`.
    pub error: Option<String>,
}

/// 설정 그대로 한 문장을 번역해 본다. 세션과 같은 캐시를 쓰므로 첫 번만 로드 시간이 든다.
pub fn test_translation(settings: &Settings, models_dir: &Path, cache: &LlmCache) -> TestResult {
    let started = Instant::now();
    let fail = |code: &str, text: String| TestResult {
        ok: false,
        ms: started.elapsed().as_millis() as u64,
        text,
        error: Some(code.into()),
    };
    let mut t = match build(settings, models_dir, cache) {
        Ok(Some(t)) => t,
        Ok(None) => return fail("display_mode_source", String::new()),
        Err(e) => return fail(&e, String::new()),
    };
    let req = TranslateRequest {
        text: "Good morning.".into(),
        src: "en".into(),
        tgt: resolve_tgt(settings),
        context: vec![],
    };
    match t.translate(&req) {
        Ok(text) => TestResult {
            ok: true,
            ms: started.elapsed().as_millis() as u64,
            text,
            error: None,
        },
        Err(e) => fail("translate", e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_tgt_uses_fixed_value() {
        let mut s = Settings::default();
        s.overlay.subtitle_lang = "ja".into();
        assert_eq!(resolve_tgt(&s), "ja");
    }

    #[test]
    fn resolve_tgt_system_is_a_concrete_code() {
        let s = Settings::default();
        assert!(matches!(resolve_tgt(&s).as_str(), "ko" | "en" | "ja"));
    }

    #[test]
    fn source_only_display_disables_translation() {
        let mut s = Settings::default();
        s.overlay.display_mode = "source".into();
        assert!(!enabled(&s));
        assert_eq!(label(&s), None);
        assert!(precheck(&s, Path::new("/nonexistent")).is_ok());
        assert!(build(&s, Path::new("/nonexistent"), &LlmCache::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn missing_local_model_is_reported() {
        let s = Settings::default();
        assert_eq!(
            precheck(&s, Path::new("/nonexistent")),
            Err("translation_model_missing".into())
        );
        assert_eq!(label(&s).as_deref(), Some("local:qwen3.5-2b"));
    }

    #[test]
    fn custom_provider_needs_base_url_or_key() {
        let mut s = Settings::default();
        s.translation.backend = "cloud".into();
        s.translation.cloud.provider = "custom".into();
        s.translation.cloud.base_url = String::new();
        // 키가 없으면 api_key_missing, 있으면 base_url_missing — 어느 쪽이든 실패해야 한다.
        let err = precheck(&s, Path::new("/nonexistent")).unwrap_err();
        assert!(
            matches!(err.as_str(), "api_key_missing" | "base_url_missing"),
            "{err}"
        );
        assert_eq!(label(&s).as_deref(), Some("cloud:custom/gpt-4o-mini"));
    }

    #[test]
    fn target_is_none_when_fixed_source_equals_target() {
        let mut s = Settings::default();
        s.overlay.subtitle_lang = "en".into();
        s.asr.source_lang = "en".into();
        assert_eq!(target(&s), None, "en→en 은 번역 단계를 만들지 않는다");
        assert_eq!(
            label(&s),
            None,
            "번역 단계가 없으면 translator 컬럼도 비운다"
        );
        s.asr.source_lang = "auto".into();
        assert_eq!(target(&s).as_deref(), Some("en"), "auto 는 항상 번역 단계");
        assert_eq!(label(&s).as_deref(), Some("local:qwen3.5-2b"));
        s.asr.source_lang = "ko".into();
        assert_eq!(target(&s).as_deref(), Some("en"));
        s.overlay.display_mode = "source".into();
        assert_eq!(target(&s), None, "원문만 모드는 번역 없음");
    }
}
