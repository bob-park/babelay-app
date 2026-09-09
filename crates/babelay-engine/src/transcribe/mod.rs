//! 전사기 트레이트와 공통 타입. 구현체는 `whisper`(whisper.cpp)와 `qwen3asr`(llama.cpp mtmd).

pub mod whisper;

pub use whisper::WhisperTranscriber;

/// 16 kHz 기준 최소 입력 길이. whisper-rs는 빈 입력을 거부하고 whisper.cpp는 100ms
/// 미만 버퍼를 건너뛴다. 경로를 단순하게 유지하려고 1초까지 패딩한다.
pub(crate) const MIN_SAMPLES: usize = 16_000;

/// whisper.cpp 로그를 러스트 로그 훅으로 돌린다. 프로세스당 한 번만 호출한다.
/// (호스트가 whisper-rs 에 직접 의존하지 않도록 감싼다.)
pub fn install_logging_hooks() {
    whisper_rs::install_logging_hooks();
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Segment {
    pub text: String,
    pub lang: String,
    pub t0_ms: u64,
    pub t1_ms: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum TranscribeError {
    #[error("model load failed: {0}")]
    Load(String),
    #[error("inference failed: {0}")]
    Inference(String),
}

pub trait Transcriber: Send {
    fn transcribe(
        &mut self,
        pcm16k: &[f32],
        lang: Option<&str>,
    ) -> Result<Vec<Segment>, TranscribeError>;
}

pub(crate) fn join(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn join_segments_skips_blank_and_trims() {
        assert_eq!(super::join(&["  Hello", "", " world. "]), "Hello world.");
    }
}
