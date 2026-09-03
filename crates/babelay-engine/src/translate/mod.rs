//! 번역 트레이트와 공통 후처리. 구현체는 `local`(llama.cpp)과 `cloud`(HTTP API).

pub mod cloud;
pub mod local;
pub mod prompt;
pub use prompt::*;

#[derive(Debug, Clone)]
pub struct TranslateRequest {
    pub text: String,
    /// 원어 코드(`ko|en|ja`). Whisper가 감지한 값.
    pub src: String,
    pub tgt: String,
    /// 직전 확정 원문(최대 2개). 대명사·문맥 보정용.
    pub context: Vec<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum TranslateError {
    #[error("load: {0}")]
    Load(String),
    #[error("request: {0}")]
    Request(String),
    /// HTTP 상태가 있는 실패. 재시도 판단(5xx)이 문자열이 아니라 상태를 본다.
    #[error("http {0}: {1}")]
    Http(u16, String),
    #[error("rate limited")]
    RateLimited,
    #[error("auth")]
    Auth,
    #[error("timeout")]
    Timeout,
    #[error("empty result")]
    Empty,
}

/// 동기 번역기. 엔진의 번역 워커 스레드에서 호출된다.
pub trait Translator: Send {
    fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError>;
    fn name(&self) -> &str;
}

/// 모델 출력 정리: `<think>…</think>` 제거, 앞뒤 따옴표·공백 제거, 줄바꿈과 연속 공백은 한 칸으로,
/// 끝에 남은 `/no_think` 제거.
pub fn postprocess(raw: &str) -> String {
    let mut s = raw.to_string();
    while let Some(open) = s.find("<think>") {
        match s[open..].find("</think>") {
            Some(rel) => s.replace_range(open..open + rel + "</think>".len(), ""),
            None => s.truncate(open),
        }
    }
    let trimmed = s.trim_matches(|c: char| {
        matches!(
            c,
            '"' | '“' | '”' | '「' | '」' | '\'' | ' ' | '\n' | '\r' | '\t'
        )
    });
    let joined = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    // 채팅 템플릿이 소비하지 않은 Qwen3 스위치가 답변에 섞여 나오는 일이 있다.
    joined
        .strip_suffix("/no_think")
        .unwrap_or(&joined)
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_names() {
        assert_eq!(lang_name("ko"), "Korean");
        assert_eq!(lang_name("ja"), "Japanese");
        assert_eq!(lang_name("xx"), "the target language");
    }

    #[test]
    fn system_prompt_names_target() {
        assert!(system_prompt("ko").contains("Korean"));
    }

    #[test]
    fn user_prompt_includes_context_block_only_when_present() {
        let no = TranslateRequest {
            text: "Hello".into(),
            src: "en".into(),
            tgt: "ko".into(),
            context: vec![],
        };
        assert_eq!(user_prompt(&no), "Hello");
        let yes = TranslateRequest {
            context: vec!["A.".into(), "B.".into()],
            ..no.clone()
        };
        let p = user_prompt(&yes);
        assert!(p.contains("- A.") && p.contains("- B.") && p.ends_with("Hello"));
    }

    #[test]
    fn postprocess_strips_think_quotes_and_newlines() {
        assert_eq!(
            postprocess("<think>reasoning\nmore</think>\n\"안녕하세요\"\n"),
            "안녕하세요"
        );
        assert_eq!(postprocess("첫 줄\n둘째 줄"), "첫 줄 둘째 줄");
        assert_eq!(postprocess("   "), "");
        assert_eq!(postprocess("<think>unterminated"), "");
        assert_eq!(postprocess("안녕하세요 /no_think"), "안녕하세요");
    }
}
