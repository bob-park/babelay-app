//! llama.cpp 백엔드 핸들. 로컬 번역기와 Qwen3-ASR 전사기가 공유한다.
use llama_cpp_2::llama_backend::LlamaBackend;
use std::sync::OnceLock;

/// 프로세스당 한 번만 초기화한다(두 번 init 하면 llama.cpp 가 에러를 낸다).
pub(crate) fn backend() -> &'static LlamaBackend {
    static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
    BACKEND.get_or_init(|| LlamaBackend::init().expect("llama backend"))
}
