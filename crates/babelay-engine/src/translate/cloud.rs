//! 클라우드 번역기: OpenAI 호환 채팅, Anthropic, Gemini, DeepL. 모두 블로킹 HTTP.
use super::{
    postprocess, system_prompt, user_prompt, TranslateError, TranslateRequest, Translator,
};
use reqwest::blocking::{Client, Response};
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_DELAYS_MS: [u64; 2] = [200, 600];

fn http() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(TIMEOUT)
            .build()
            .expect("reqwest client")
    })
}

fn send_err(e: reqwest::Error) -> TranslateError {
    if e.is_timeout() {
        TranslateError::Timeout
    } else {
        TranslateError::Request(e.to_string())
    }
}

/// HTTP 상태를 오류로. 상태는 `Http` 에 그대로 담아 `retryable` 이 문자열을 보지 않게 한다.
pub(crate) fn map_status(status: u16, body: &str) -> TranslateError {
    match status {
        401 | 403 => TranslateError::Auth,
        429 => TranslateError::RateLimited,
        _ => TranslateError::Http(status, body.chars().take(200).collect()),
    }
}

fn retryable(e: &TranslateError) -> bool {
    match e {
        TranslateError::RateLimited | TranslateError::Timeout => true,
        TranslateError::Http(status, _) => (500..600).contains(status),
        _ => false,
    }
}

/// 429 / 5xx / 타임아웃이면 200ms, 600ms 뒤 최대 2회 재시도. 그 외 오류는 즉시 반환.
pub fn with_retry<T>(
    mut f: impl FnMut() -> Result<T, TranslateError>,
) -> Result<T, TranslateError> {
    let mut attempt = 0;
    loop {
        match f() {
            Err(e) if attempt < RETRY_DELAYS_MS.len() && retryable(&e) => {
                std::thread::sleep(Duration::from_millis(RETRY_DELAYS_MS[attempt]));
                attempt += 1;
            }
            r => return r,
        }
    }
}

/// 응답을 JSON 으로 읽고 `pointer` 위치의 문자열을 후처리해 돌려준다.
fn extract(resp: Response, pointer: &str) -> Result<String, TranslateError> {
    let status = resp.status().as_u16();
    let body = resp.text().map_err(send_err)?;
    if !(200..300).contains(&status) {
        return Err(map_status(status, &body));
    }
    let v: Value =
        serde_json::from_str(&body).map_err(|e| TranslateError::Request(format!("json: {e}")))?;
    let text = postprocess(v.pointer(pointer).and_then(Value::as_str).unwrap_or(""));
    if text.is_empty() {
        Err(TranslateError::Empty)
    } else {
        Ok(text)
    }
}

/// OpenAI Chat Completions 와 같은 형식을 쓰는 모든 서버(OpenAI, 사용자 지정 base_url).
pub struct OpenAiCompatible {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Default for OpenAiCompatible {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            model: "gpt-4o-mini".into(),
        }
    }
}

impl OpenAiCompatible {
    fn once(&self, req: &TranslateRequest) -> Result<String, TranslateError> {
        let resp = http()
            .post(format!(
                "{}/chat/completions",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": self.model,
                "temperature": 0,
                "max_tokens": 512,
                "messages": [
                    {"role": "system", "content": system_prompt(&req.tgt)},
                    {"role": "user", "content": user_prompt(req)},
                ],
            }))
            .send()
            .map_err(send_err)?;
        extract(resp, "/choices/0/message/content")
    }
}

impl Translator for OpenAiCompatible {
    fn name(&self) -> &str {
        "openai"
    }
    fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
        with_retry(|| self.once(req))
    }
}

pub struct Anthropic {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Default for Anthropic {
    fn default() -> Self {
        Self {
            base_url: "https://api.anthropic.com".into(),
            api_key: String::new(),
            model: "claude-haiku-4-5-20251001".into(),
        }
    }
}

impl Anthropic {
    fn once(&self, req: &TranslateRequest) -> Result<String, TranslateError> {
        let resp = http()
            .post(format!(
                "{}/v1/messages",
                self.base_url.trim_end_matches('/')
            ))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": self.model,
                "max_tokens": 512,
                "system": system_prompt(&req.tgt),
                "messages": [{"role": "user", "content": user_prompt(req)}],
            }))
            .send()
            .map_err(send_err)?;
        extract(resp, "/content/0/text")
    }
}

impl Translator for Anthropic {
    fn name(&self) -> &str {
        "anthropic"
    }
    fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
        with_retry(|| self.once(req))
    }
}

pub struct Gemini {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Default for Gemini {
    fn default() -> Self {
        Self {
            base_url: "https://generativelanguage.googleapis.com".into(),
            api_key: String::new(),
            model: "gemini-2.5-flash".into(),
        }
    }
}

impl Gemini {
    fn once(&self, req: &TranslateRequest) -> Result<String, TranslateError> {
        let resp = http()
            .post(format!(
                "{}/v1beta/models/{}:generateContent",
                self.base_url.trim_end_matches('/'),
                self.model
            ))
            .query(&[("key", self.api_key.as_str())])
            .json(&json!({
                "system_instruction": {"parts": [{"text": system_prompt(&req.tgt)}]},
                "contents": [{"role": "user", "parts": [{"text": user_prompt(req)}]}],
                "generationConfig": {"maxOutputTokens": 512},
            }))
            .send()
            .map_err(send_err)?;
        extract(resp, "/candidates/0/content/parts/0/text")
    }
}

impl Translator for Gemini {
    fn name(&self) -> &str {
        "gemini"
    }
    fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
        with_retry(|| self.once(req))
    }
}

pub struct DeepL {
    pub base_url: String,
    pub api_key: String,
}

impl Default for DeepL {
    fn default() -> Self {
        Self {
            base_url: "https://api.deepl.com".into(),
            api_key: String::new(),
        }
    }
}

impl DeepL {
    /// 무료 키(`:fx` 접미사)는 free 엔드포인트, 그 외는 pro.
    pub fn new(api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let base_url = if api_key.ends_with(":fx") {
            "https://api-free.deepl.com"
        } else {
            "https://api.deepl.com"
        };
        Self {
            base_url: base_url.into(),
            api_key,
        }
    }

    fn once(&self, req: &TranslateRequest) -> Result<String, TranslateError> {
        // DeepL 은 타겟으로 맨 EN 을 더 이상 권하지 않는다(EN-US / EN-GB).
        let tgt = match req.tgt.as_str() {
            "en" => "EN-US".to_string(),
            other => other.to_ascii_uppercase(),
        };
        let mut form = vec![("text", req.text.clone()), ("target_lang", tgt)];
        if matches!(req.src.as_str(), "ko" | "en" | "ja") {
            form.push(("source_lang", req.src.to_ascii_uppercase()));
        }
        if !req.context.is_empty() {
            form.push(("context", req.context.join(" ")));
        }
        let resp = http()
            .post(format!(
                "{}/v2/translate",
                self.base_url.trim_end_matches('/')
            ))
            .header("authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .form(&form)
            .send()
            .map_err(send_err)?;
        extract(resp, "/translations/0/text")
    }
}

impl Translator for DeepL {
    fn name(&self) -> &str {
        "deepl"
    }
    fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
        with_retry(|| self.once(req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    fn req(text: &str, src: &str, tgt: &str) -> TranslateRequest {
        TranslateRequest {
            text: text.into(),
            src: src.into(),
            tgt: tgt.into(),
            context: vec![],
        }
    }

    #[test]
    fn with_retry_gives_up_after_three_attempts() {
        let mut calls = 0;
        let r: Result<(), _> = with_retry(|| {
            calls += 1;
            Err(TranslateError::RateLimited)
        });
        assert!(matches!(r, Err(TranslateError::RateLimited)));
        assert_eq!(calls, 3);

        let mut calls = 0;
        let r: Result<(), _> = with_retry(|| {
            calls += 1;
            Err(TranslateError::Auth)
        });
        assert!(matches!(r, Err(TranslateError::Auth)));
        assert_eq!(calls, 1, "Auth 는 재시도하지 않는다");
    }

    #[test]
    fn map_status_classifies() {
        assert!(matches!(map_status(401, ""), TranslateError::Auth));
        assert!(matches!(map_status(429, ""), TranslateError::RateLimited));
        assert!(retryable(&map_status(503, "down")));
        assert!(!retryable(&map_status(400, "bad")));
    }

    // ---- OpenAI ----

    #[test]
    fn openai_parses_content_and_sends_bearer() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST)
                .path("/v1/chat/completions")
                .header("authorization", "Bearer k")
                .body_contains("Korean")
                .body_contains("\"max_tokens\":512");
            t.status(200)
                .json_body(json!({"choices":[{"message":{"content":"안녕"}}]}));
        });
        let mut c = OpenAiCompatible {
            base_url: s.url("/v1"),
            api_key: "k".into(),
            model: "gpt-4o-mini".into(),
        };
        assert_eq!(c.translate(&req("Hello", "en", "ko")).unwrap(), "안녕");
        m.assert();
    }

    #[test]
    fn openai_retries_on_429() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST).path("/v1/chat/completions");
            t.status(429);
        });
        let mut c = OpenAiCompatible {
            base_url: s.url("/v1"),
            api_key: "k".into(),
            model: "m".into(),
        };
        assert!(matches!(
            c.translate(&req("Hello", "en", "ko")),
            Err(TranslateError::RateLimited)
        ));
        assert_eq!(m.hits(), 3);
    }

    #[test]
    fn openai_401_is_auth() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST).path("/v1/chat/completions");
            t.status(401);
        });
        let mut c = OpenAiCompatible {
            base_url: s.url("/v1"),
            api_key: "bad".into(),
            model: "m".into(),
        };
        assert!(matches!(
            c.translate(&req("Hello", "en", "ko")),
            Err(TranslateError::Auth)
        ));
        assert_eq!(m.hits(), 1);
    }

    // ---- Anthropic ----

    #[test]
    fn anthropic_parses_text_and_sends_headers() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST)
                .path("/v1/messages")
                .header("x-api-key", "k")
                .header("anthropic-version", "2023-06-01")
                .body_contains("Korean");
            t.status(200)
                .json_body(json!({"content":[{"type":"text","text":"안녕"}]}));
        });
        let mut c = Anthropic {
            base_url: s.url(""),
            api_key: "k".into(),
            model: "claude".into(),
        };
        assert_eq!(c.translate(&req("Hello", "en", "ko")).unwrap(), "안녕");
        m.assert();
    }

    #[test]
    fn anthropic_retries_on_429() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST).path("/v1/messages");
            t.status(429);
        });
        let mut c = Anthropic {
            base_url: s.url(""),
            api_key: "k".into(),
            model: "m".into(),
        };
        assert!(matches!(
            c.translate(&req("Hello", "en", "ko")),
            Err(TranslateError::RateLimited)
        ));
        assert_eq!(m.hits(), 3);
    }

    #[test]
    fn anthropic_401_is_auth() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST).path("/v1/messages");
            t.status(401);
        });
        let mut c = Anthropic {
            base_url: s.url(""),
            api_key: "bad".into(),
            model: "m".into(),
        };
        assert!(matches!(
            c.translate(&req("Hello", "en", "ko")),
            Err(TranslateError::Auth)
        ));
        assert_eq!(m.hits(), 1);
    }

    // ---- Gemini ----

    #[test]
    fn gemini_parses_text_and_sends_key_query() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST)
                .path("/v1beta/models/gemini-2.5-flash:generateContent")
                .query_param("key", "k")
                .body_contains("Korean")
                .body_contains("\"maxOutputTokens\":512");
            t.status(200).json_body(
                json!({"candidates":[{"content":{"parts":[{"text":"안녕"}],"role":"model"}}]}),
            );
        });
        let mut c = Gemini {
            base_url: s.url(""),
            api_key: "k".into(),
            model: "gemini-2.5-flash".into(),
        };
        assert_eq!(c.translate(&req("Hello", "en", "ko")).unwrap(), "안녕");
        m.assert();
    }

    #[test]
    fn gemini_retries_on_429() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST).path("/v1beta/models/g:generateContent");
            t.status(429);
        });
        let mut c = Gemini {
            base_url: s.url(""),
            api_key: "k".into(),
            model: "g".into(),
        };
        assert!(matches!(
            c.translate(&req("Hello", "en", "ko")),
            Err(TranslateError::RateLimited)
        ));
        assert_eq!(m.hits(), 3);
    }

    #[test]
    fn gemini_403_is_auth() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST).path("/v1beta/models/g:generateContent");
            t.status(403);
        });
        let mut c = Gemini {
            base_url: s.url(""),
            api_key: "bad".into(),
            model: "g".into(),
        };
        assert!(matches!(
            c.translate(&req("Hello", "en", "ko")),
            Err(TranslateError::Auth)
        ));
        assert_eq!(m.hits(), 1);
    }

    // ---- DeepL ----

    #[test]
    fn deepl_parses_text_and_sends_form() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST)
                .path("/v2/translate")
                .header("authorization", "DeepL-Auth-Key k:fx")
                .body_contains("target_lang=KO")
                .body_contains("source_lang=EN")
                .body_contains("context=Prev");
            t.status(200).json_body(
                json!({"translations":[{"detected_source_language":"EN","text":"안녕"}]}),
            );
        });
        let mut c = DeepL {
            base_url: s.url(""),
            api_key: "k:fx".into(),
        };
        let with_context = TranslateRequest {
            context: vec!["Prev one".into(), "Prev two".into()],
            ..req("Hello", "en", "ko")
        };
        assert_eq!(c.translate(&with_context).unwrap(), "안녕");
        m.assert();
    }

    #[test]
    fn deepl_retries_on_429() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST).path("/v2/translate");
            t.status(429);
        });
        let mut c = DeepL {
            base_url: s.url(""),
            api_key: "k".into(),
        };
        assert!(matches!(
            c.translate(&req("Hello", "en", "ko")),
            Err(TranslateError::RateLimited)
        ));
        assert_eq!(m.hits(), 3);
    }

    #[test]
    fn deepl_403_is_auth() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST).path("/v2/translate");
            t.status(403);
        });
        let mut c = DeepL {
            base_url: s.url(""),
            api_key: "bad".into(),
        };
        assert!(matches!(
            c.translate(&req("Hello", "en", "ko")),
            Err(TranslateError::Auth)
        ));
        assert_eq!(m.hits(), 1);
    }

    #[test]
    fn deepl_uses_en_us_for_english_targets() {
        let s = MockServer::start();
        let m = s.mock(|w, t| {
            w.method(POST)
                .path("/v2/translate")
                .body_contains("target_lang=EN-US");
            t.status(200)
                .json_body(json!({"translations":[{"text":"Hi"}]}));
        });
        let mut c = DeepL {
            base_url: s.url(""),
            api_key: "k".into(),
        };
        assert_eq!(c.translate(&req("안녕", "ko", "en")).unwrap(), "Hi");
        m.assert();
    }

    #[test]
    fn deepl_picks_free_endpoint_for_fx_keys() {
        assert_eq!(DeepL::new("abc:fx").base_url, "https://api-free.deepl.com");
        assert_eq!(DeepL::new("abc").base_url, "https://api.deepl.com");
    }
}
