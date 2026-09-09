//! 번역 프롬프트 조립. 로컬 LLM과 클라우드 채팅 API가 공유한다.

use super::TranslateRequest;

/// 언어 코드 → 영어 언어명. 알 수 없는 코드는 일반 표현으로.
pub fn lang_name(code: &str) -> &'static str {
    match code {
        "ko" => "Korean",
        "en" => "English",
        "ja" => "Japanese",
        _ => "the target language",
    }
}

pub fn system_prompt(tgt: &str) -> String {
    format!(
        "You are a subtitle translator. Translate the user's text into {}. \
         Output only the translation, one line, no quotes, no explanations.",
        lang_name(tgt)
    )
}

/// 컨텍스트(직전 원문)가 있으면 번역 대상과 구분해 넣고, 없으면 원문 그대로.
pub fn user_prompt(req: &TranslateRequest) -> String {
    if req.context.is_empty() {
        return req.text.clone();
    }
    let mut p = String::from("Previous lines (for context, do not translate):\n");
    for line in &req.context {
        p.push_str("- ");
        p.push_str(line);
        p.push('\n');
    }
    p.push_str("\nTranslate:\n");
    p.push_str(&req.text);
    p
}

/// HY-MT(Tencent) 전용. 번역 명령 모델이라 system 없이 user 한 턴으로 보내고 문맥은 붙이지 않는다.
/// ponytail: 문맥 없이 번역. 대명사 오역이 실측되면 Hy-MT2 의 [Background Information] 블록으로 확장.
pub fn hy_mt_prompt(req: &TranslateRequest) -> String {
    format!(
        "Translate the following text into {}. Note that you should only output the translated \
         result without any additional explanation:\n\n{}",
        lang_name(&req.tgt),
        req.text
    )
}
