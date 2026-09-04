//! 로컬 LLM 번역기(llama.cpp). 컨텍스트 하나를 재사용하며 요청마다 KV 캐시만 비우고 greedy 로 디코딩한다.
use crate::translate::{
    postprocess, system_prompt, user_prompt, TranslateError, TranslateRequest, Translator,
};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::OnceLock;

/// 프로세스당 한 번만 초기화한다(두 번 init 하면 llama.cpp 가 에러를 낸다).
fn backend() -> &'static LlamaBackend {
    static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
    BACKEND.get_or_init(|| LlamaBackend::init().expect("llama backend"))
}

/// Qwen3 계열은 thinking 을 끄기 위해 어시스턴트 턴을 빈 think 블록으로 미리 채운다. 파일명으로 판별한다.
pub(crate) fn is_qwen3(p: &Path) -> bool {
    p.file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase().contains("qwen3"))
        .unwrap_or(false)
}

/// 컨텍스트 길이. 프롬프트(시스템+직전 문맥+원문)와 생성분(입력의 3배)이 이 안에 들어간다.
const N_CTX: u32 = 4096;

pub struct LocalLlm {
    // `ctx` 가 `model` 을 빌린다. 필드는 선언 순서대로 드롭되므로 ctx 가 먼저 사라지고,
    // model 은 Box 라 힙 주소가 고정이다. 그래서 수명을 'static 으로 늘려도 안전하다.
    ctx: LlamaContext<'static>,
    model: Box<LlamaModel>,
    qwen3: bool,
    pub gpu_active: bool,
}

// SAFETY: llama_context 는 스레드 친화성이 없고 &mut 로만 쓰므로 한 번에 한 스레드만 접근한다.
unsafe impl Send for LocalLlm {}

impl LocalLlm {
    /// GPU 로드가 실패하면 CPU 로 한 번 더 시도한다. 두 번째 값은 그 폴백 여부.
    pub fn load(path: &Path, use_gpu: bool) -> Result<(Self, bool), TranslateError> {
        let try_load = |layers: u32| {
            LlamaModel::load_from_file(
                backend(),
                path,
                &LlamaModelParams::default().with_n_gpu_layers(layers),
            )
        };
        let (model, fell_back) = match try_load(if use_gpu { 1000 } else { 0 }) {
            Ok(m) => (m, false),
            Err(e) if use_gpu => {
                let m =
                    try_load(0).map_err(|e2| TranslateError::Load(format!("{e}; cpu: {e2}")))?;
                (m, true)
            }
            Err(e) => return Err(TranslateError::Load(e.to_string())),
        };
        let threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4)
            .min(8);
        let model = Box::new(model);
        let params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(N_CTX))
            .with_n_threads(threads)
            .with_n_threads_batch(threads);
        // SAFETY: struct 주석 참고. model 은 Box 안에 있고 ctx 보다 늦게 드롭된다.
        let model_ref: &'static LlamaModel = unsafe { &*(&*model as *const LlamaModel) };
        let ctx = model_ref
            .new_context(backend(), params)
            .map_err(|e| TranslateError::Load(e.to_string()))?;
        Ok((
            Self {
                ctx,
                model,
                qwen3: is_qwen3(path),
                gpu_active: use_gpu && !fell_back,
            },
            fell_back,
        ))
    }

    /// 모델 채팅 템플릿으로 system+user 를 렌더한다. 템플릿이 없으면 ChatML.
    /// Qwen3 계열은 `/no_think` 를 무시하고(Qwen3.5 실측) 사고 블록 안에서 생성 예산을 다 써 버리므로,
    /// 어시스턴트 턴을 빈 `<think></think>` 로 미리 채워 사고를 끈다(템플릿의 enable_thinking=false 와 같은 효과).
    fn render(&self, req: &TranslateRequest) -> Result<String, TranslateError> {
        let user = user_prompt(req);
        let sys = system_prompt(&req.tgt);
        let msgs = vec![
            LlamaChatMessage::new("system".into(), sys.clone())
                .map_err(|e| TranslateError::Request(e.to_string()))?,
            LlamaChatMessage::new("user".into(), user.clone())
                .map_err(|e| TranslateError::Request(e.to_string()))?,
        ];
        let mut prompt = match self.model.chat_template(None) {
            Ok(tmpl) => self
                .model
                .apply_chat_template(&tmpl, &msgs, true)
                .map_err(|e| TranslateError::Request(e.to_string()))?,
            Err(_) => format!(
                "<|im_start|>system\n{sys}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n"
            ),
        };
        if self.qwen3 && !prompt.trim_end().ends_with("</think>") {
            prompt.push_str("<think>\n\n</think>\n\n");
        }
        Ok(prompt)
    }
}

impl Translator for LocalLlm {
    fn name(&self) -> &str {
        "local"
    }

    fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
        let prompt = self.render(req)?;
        let tokens = self
            .model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| TranslateError::Request(e.to_string()))?;
        if tokens.is_empty() {
            return Err(TranslateError::Empty);
        }
        // 번역은 원문보다 크게 길어지지 않는다. 입력 토큰의 3배(최소 32)에서 끊되 컨텍스트를 넘기지 않는다.
        let max_new = (tokens.len() * 3)
            .max(32)
            .min((N_CTX as usize).saturating_sub(tokens.len() + 8));
        if max_new == 0 {
            return Err(TranslateError::Request("prompt exceeds context".into()));
        }
        let ctx = &mut self.ctx;
        ctx.clear_kv_cache();

        let mut batch = LlamaBatch::new(tokens.len().max(1), 1);
        let last = tokens.len() - 1;
        for (i, t) in tokens.iter().enumerate() {
            batch
                .add(*t, i as i32, &[0], i == last)
                .map_err(|e| TranslateError::Request(e.to_string()))?;
        }
        ctx.decode(&mut batch)
            .map_err(|e| TranslateError::Request(e.to_string()))?;

        let mut sampler = LlamaSampler::greedy();
        // 토큰 하나가 UTF-8 문자 중간에서 끊길 수 있어 상태 있는 디코더로 이어 붙인다.
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut out = String::new();
        for pos in (tokens.len() as i32..).take(max_new) {
            let tok = sampler.sample(ctx, batch.n_tokens() - 1);
            if self.model.is_eog_token(tok) {
                break;
            }
            out.push_str(
                &self
                    .model
                    .token_to_piece(tok, &mut decoder, false, None)
                    .unwrap_or_default(),
            );
            batch.clear();
            batch
                .add(tok, pos, &[0], true)
                .map_err(|e| TranslateError::Request(e.to_string()))?;
            ctx.decode(&mut batch)
                .map_err(|e| TranslateError::Request(e.to_string()))?;
        }
        let text = postprocess(&out);
        if text.is_empty() {
            Err(TranslateError::Empty)
        } else {
            Ok(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::translate::Translator;

    #[test]
    #[ignore = "needs BABELAY_TEST_LLM=<gguf>"]
    fn translates_english_to_korean() {
        let path = std::env::var("BABELAY_TEST_LLM").unwrap();
        let started = std::time::Instant::now();
        let (mut t, _) = super::LocalLlm::load(std::path::Path::new(&path), true).unwrap();
        let req = crate::translate::TranslateRequest {
            text: "Good morning, everyone.".into(),
            src: "en".into(),
            tgt: "ko".into(),
            context: vec![],
        };
        let out = t.translate(&req).unwrap();
        println!("{out} ({} ms)", started.elapsed().as_millis());
        assert!(!out.is_empty());
        assert!(!out.contains("<think>"));
        assert!(out.chars().any(|c| ('가'..='힣').contains(&c)));
        // 같은 컨텍스트로 두 번째 요청: KV 캐시가 비워져 첫 결과가 섞이지 않아야 한다.
        let started = std::time::Instant::now();
        let req2 = crate::translate::TranslateRequest {
            text: "The meeting is over.".into(),
            ..req
        };
        let out2 = t.translate(&req2).unwrap();
        println!("{out2} ({} ms)", started.elapsed().as_millis());
        assert!(out2.chars().any(|c| ('가'..='힣').contains(&c)));
        assert_ne!(out, out2);
    }

    #[test]
    fn qwen_detection_by_filename() {
        assert!(super::is_qwen3(std::path::Path::new(
            "/x/Qwen3-4B-Q4_K_M.gguf"
        )));
        assert!(super::is_qwen3(std::path::Path::new(
            "/x/Qwen3.5-2B-Q4_K_M.gguf"
        )));
        assert!(!super::is_qwen3(std::path::Path::new(
            "/x/gemma-3-1b-it-Q4_K_M.gguf"
        )));
    }
}
