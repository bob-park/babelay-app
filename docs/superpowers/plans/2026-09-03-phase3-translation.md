# Babelay 3단계: 번역 구현 계획 (as-built)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> 기록 메모(2026-09-04): 이 계획은 2026-09-03 밤 두 세션이 같은 파일명으로 각자 작성했다. 한 세션의 8-태스크 계획이 다른 세션의 7-태스크 계획을 덮어썼고, 코드는 7-태스크 계획의 태스크 브리프대로 구현됐다. 아래는 그 브리프(Task 2–7 원문)와 코드에서 복원한 Task 1로 재구성한 "실제로 만든 것"의 기록이다. 8-태스크 초안은 `.superpowers/sdd/2026-09-03-phase3-translation/plan-8task-unbuilt.md`(git 무시)에 남겼다.

**Goal:** 확정된 전사 조각(`Final`)을 로컬 LLM(llama.cpp) 또는 클라우드 API(OpenAI 호환·Anthropic·Gemini·DeepL)로 번역해 오버레이·라이브·히스토리에 원문+번역 한 세트로 보여주고, API 키는 OS 자격 증명 저장소에 두며, 설정 › 번역에서 키 저장·연결 테스트를 제공한다.

**Architecture:** 엔진에 `translate` 모듈(동기 `Translator` trait, `prompt.rs`, `local.rs`=llama-cpp-2, `cloud.rs`=어댑터 4종)을 두고 `engine.rs`의 전사 스레드 뒤에 번역 워커 스레드를 붙여 `EngineEvent::Translated`를 낸다. `src-tauri`는 `keys.rs`(keyring)와 `translator.rs`(설정 → 번역기 조립·연결 테스트)로 세션에 번역기를 붙이고, `history.rs`가 `Translated`를 행에 써 넣는다(`segments_au` FTS 트리거). 프론트는 `Final`에 번역을 붙이고 `pairForOverlay`가 같은 id의 번역을 3초까지 기다렸다가 두 줄을 함께 그린다.

**Tech Stack:** llama-cpp-2 0.1.156(`default-features = false`, features `common` + `metal`/`cuda`), reqwest 0.12 blocking(rustls, json), keyring, httpmock 0.7(테스트), Tauri 2, React 19, zustand 5, vitest 4.

**Spec:** `docs/superpowers/specs/2026-09-02-babelay-design.md` §3.1, §4.3, §4.4, §4.5, §6, §7.3, §7.4, §8, §11 item 3.

## Global Constraints

- 셸에 mise가 활성화되어 있지 않으면 `mise exec -- cargo/yarn …`. 게이트: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `yarn tsc --noEmit`, `yarn test`, `yarn build`.
- `babelay-engine`은 Tauri에 의존하지 않는다. 엔진의 유일한 출력은 `EngineEvent` 채널.
- 클라우드: 타임아웃 10초, 429/5xx/타임아웃은 2회 재시도, 401/403은 즉시 실패. 실패한 조각은 원문만 보여주고 넘어간다.
- 번역 건너뛰기: 조각의 원어 == 자막 언어, 또는 표시 모드 == `source`.
- API 키는 설정 파일에 절대 쓰지 않는다. keyring 서비스명 `com.babelay.app`, 계정명 = 프로바이더 id.
- 설정 스키마 변경 없음. GPU 토글은 전사·번역 공용(`asr.gpu`).
- 디자인: daisyUI 5만, 아이콘은 인라인 SVG, 초록은 채우기+검정 글자, 안내 문장 없음, 세 로케일 키 집합 동일.
- 커밋 접두어 `feat:`/`fix:`/`test:`/`docs:`, 트레일러 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` + `Claude-Session: https://claude.ai/code/session_01TwrgRuGibDouRfH5p4qmoq`.

---

### Task 1: 번역 trait · 프롬프트 · 후처리 (복원)

**Files:**
- Create: `crates/babelay-engine/src/translate/mod.rs`, `crates/babelay-engine/src/translate/prompt.rs`
- Modify: `crates/babelay-engine/src/lib.rs` (`pub mod translate;`)

**Interfaces (produces):**
```rust
pub struct TranslateRequest { pub text: String, pub src: String, pub tgt: String, pub context: Vec<String> }
pub enum TranslateError { Load(String), Request(String), RateLimited, Auth, Timeout, Empty }
pub trait Translator: Send { fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError>; fn name(&self) -> &str; }
pub fn lang_name(code: &str) -> &'static str;     // ko/en/ja → 영어 이름, 그 외 "the target language"
pub fn system_prompt(tgt: &str) -> String;         // 자막 번역가 지시문
pub fn user_prompt(req: &TranslateRequest) -> String; // 컨텍스트가 있으면 "Previous lines…" 블록 + "Translate:" + 본문
pub fn postprocess(raw: &str) -> String;           // <think>…</think> 제거, 따옴표·공백 정리, 줄바꿈→공백
```

- [x] 테스트 4개(`lang_names`, `system_prompt_names_target`, `user_prompt_includes_context_block_only_when_present`, `postprocess_strips_think_quotes_and_newlines`) → 구현 → `cargo test -p babelay-engine translate`.

---

### Task 2: 로컬 LLM 번역기 (llama-cpp-2)


**Files:**
- Create: `crates/babelay-engine/src/translate/local.rs`
- Modify: `crates/babelay-engine/Cargo.toml`, `crates/babelay-engine/src/translate/mod.rs` (`pub mod local;`)

**Interfaces (produces):**
- `pub struct LocalLlm { … pub gpu_active: bool }`
- `impl LocalLlm { pub fn load(path: &Path, use_gpu: bool) -> Result<(Self, bool /*fell_back*/), TranslateError>; }`
- `impl Translator for LocalLlm` (`name()` = "local")
- `pub(crate) fn is_qwen3(path: &Path) -> bool` (파일명에 "qwen3" 포함, 대소문자 무시)

**규칙:** `LlamaBackend::init()`은 프로세스당 한 번(`OnceLock`). `LlamaModelParams::default().with_n_gpu_layers(if use_gpu {1000} else {0})`; GPU 로드 실패 시 0으로 재시도(`fell_back = true`). 요청마다 새 컨텍스트(`n_ctx` = min(4096, max(512, 입력토큰 + max_new + 8))), `n_threads` = min(cores, 8). 프롬프트는 모델 채팅 템플릿(`model.chat_template(None)`; 실패 시 ChatML 문자열)으로 system+user 메시지를 렌더하고, Qwen3 계열이면 유저 메시지 끝에 ` /no_think`. 샘플러 `LlamaSampler::greedy()`. 최대 생성 토큰 = max(32, 입력 토큰 수 × 3), `is_eog_token`에서 중단. 결과는 `postprocess`; 빈 문자열이면 `TranslateError::Empty`.

- [ ] **Step 1: Cargo**

```toml
[dependencies]
llama-cpp-2 = { version = "0.1.156", default-features = false, features = ["common"] }

[features]
metal = ["whisper-rs/metal", "llama-cpp-2/metal"]
cuda = ["whisper-rs/cuda", "llama-cpp-2/cuda"]
```
(`openmp` 기본 feature는 macOS clang에서 빌드가 깨질 수 있어 끈다. 필요하면 `cuda` feature에만 `"llama-cpp-2/openmp"` 추가.)

- [ ] **Step 2: 테스트**

```rust
#[cfg(test)]
mod tests {
    use crate::translate::Translator;
    #[test]
    #[ignore = "needs BABELAY_TEST_LLM=<gguf>"]
    fn translates_english_to_korean() {
        let path = std::env::var("BABELAY_TEST_LLM").unwrap();
        let (mut t, _) = super::LocalLlm::load(std::path::Path::new(&path), true).unwrap();
        let req = crate::translate::TranslateRequest { text: "Good morning, everyone.".into(), src: "en".into(), tgt: "ko".into(), context: vec![] };
        let out = t.translate(&req).unwrap();
        println!("{out}");
        assert!(!out.is_empty() && !out.contains("<think>") && out.chars().any(|c| ('가'..='힣').contains(&c)));
    }
    #[test]
    fn qwen_detection_by_filename() {
        assert!(super::is_qwen3(std::path::Path::new("/x/Qwen3-4B-Q4_K_M.gguf")));
        assert!(super::is_qwen3(std::path::Path::new("/x/Qwen3.5-2B-Q4_K_M.gguf")));
        assert!(!super::is_qwen3(std::path::Path::new("/x/gemma-3-1b-it-Q4_K_M.gguf")));
    }
}
```

- [ ] **Step 3: 구현 골격** — API 이름은 `~/.cargo/registry/src/index.crates.io-*/llama-cpp-2-0.1.156/src/{model.rs,model/params.rs,context/params.rs,context.rs,llama_batch.rs,sampling.rs,llama_backend.rs}`에서 확인해 맞춘다(확인된 것: `LlamaModel::load_from_file(&LlamaBackend, path, &LlamaModelParams)`, `LlamaModelParams::with_n_gpu_layers(u32)`, `model.new_context(&LlamaBackend, LlamaContextParams)`, `model.chat_template(Option<&str>) -> Result<LlamaChatTemplate,_>`, `model.apply_chat_template(&tmpl, &[LlamaChatMessage], add_ass: bool) -> Result<String,_>`, `LlamaChatMessage::new(role: String, content: String)`, `model.str_to_token(&str, AddBos::Always)`, `LlamaBatch::new(n_tokens, n_seq_max)`, `batch.add(token, pos, &[seq_id], logits)`, `batch.clear()`, `batch.n_tokens()`, `ctx.decode(&mut batch)`, `LlamaSampler::greedy()`, `sampler.sample(&ctx, idx)`, `model.is_eog_token(token)`, `model.token_to_str(token, Special::…)`, `LlamaBackend::init()`).

```rust
use crate::translate::{postprocess, system_prompt, user_prompt, TranslateError, TranslateRequest, Translator};
use llama_cpp_2::{context::params::LlamaContextParams, llama_backend::LlamaBackend, llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, AddBos, LlamaChatMessage, LlamaModel, Special}, sampling::LlamaSampler};
use std::{num::NonZeroU32, path::Path, sync::OnceLock};

static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
fn backend() -> &'static LlamaBackend { BACKEND.get_or_init(|| LlamaBackend::init().expect("llama backend")) }

pub(crate) fn is_qwen3(p: &Path) -> bool {
    p.file_name().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase().contains("qwen3")).unwrap_or(false)
}

pub struct LocalLlm { model: LlamaModel, threads: i32, qwen3: bool, pub gpu_active: bool }

impl LocalLlm {
    pub fn load(path: &Path, use_gpu: bool) -> Result<(Self, bool), TranslateError> {
        let try_load = |layers: u32| LlamaModel::load_from_file(backend(), path, &LlamaModelParams::default().with_n_gpu_layers(layers));
        let (model, fell_back) = match try_load(if use_gpu { 1000 } else { 0 }) {
            Ok(m) => (m, false),
            Err(e) if use_gpu => (try_load(0).map_err(|e2| TranslateError::Load(format!("{e}; cpu: {e2}")))?, true),
            Err(e) => return Err(TranslateError::Load(e.to_string())),
        };
        let threads = std::thread::available_parallelism().map(|n| n.get() as i32).unwrap_or(4).min(8);
        Ok((Self { model, threads, qwen3: is_qwen3(path), gpu_active: use_gpu && !fell_back }, fell_back))
    }

    fn render(&self, req: &TranslateRequest) -> Result<String, TranslateError> {
        let mut user = user_prompt(req);
        if self.qwen3 { user.push_str(" /no_think"); }
        let sys = system_prompt(&req.tgt);
        let msgs = vec![
            LlamaChatMessage::new("system".into(), sys.clone()).map_err(|e| TranslateError::Request(e.to_string()))?,
            LlamaChatMessage::new("user".into(), user.clone()).map_err(|e| TranslateError::Request(e.to_string()))?,
        ];
        match self.model.chat_template(None) {
            Ok(tmpl) => self.model.apply_chat_template(&tmpl, &msgs, true).map_err(|e| TranslateError::Request(e.to_string())),
            Err(_) => Ok(format!("<|im_start|>system\n{sys}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n")),
        }
    }
}

impl Translator for LocalLlm {
    fn name(&self) -> &str { "local" }
    fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
        let prompt = self.render(req)?;
        let tokens = self.model.str_to_token(&prompt, AddBos::Always).map_err(|e| TranslateError::Request(e.to_string()))?;
        let max_new = (tokens.len() * 3).max(32);
        let n_ctx = ((tokens.len() + max_new + 8).max(512) as u32).min(4096);
        let params = LlamaContextParams::default().with_n_ctx(NonZeroU32::new(n_ctx)).with_n_threads(self.threads);
        let mut ctx = self.model.new_context(backend(), params).map_err(|e| TranslateError::Load(e.to_string()))?;
        let mut batch = LlamaBatch::new(tokens.len() + 1, 1);
        let last = tokens.len() - 1;
        for (i, t) in tokens.iter().enumerate() {
            batch.add(*t, i as i32, &[0], i == last).map_err(|e| TranslateError::Request(e.to_string()))?;
        }
        ctx.decode(&mut batch).map_err(|e| TranslateError::Request(e.to_string()))?;
        let mut sampler = LlamaSampler::greedy();
        let mut out = String::new();
        let mut pos = tokens.len() as i32;
        for _ in 0..max_new {
            let tok = sampler.sample(&ctx, batch.n_tokens() - 1);
            if self.model.is_eog_token(tok) { break; }
            out.push_str(&self.model.token_to_str(tok, Special::Plaintext).unwrap_or_default());
            batch.clear();
            batch.add(tok, pos, &[0], true).map_err(|e| TranslateError::Request(e.to_string()))?;
            pos += 1;
            ctx.decode(&mut batch).map_err(|e| TranslateError::Request(e.to_string()))?;
        }
        let text = postprocess(&out);
        if text.is_empty() { Err(TranslateError::Empty) } else { Ok(text) }
    }
}
```
`with_n_ctx`/`with_n_threads`가 다른 시그니처면(예: `with_n_threads(i32)` vs `u32`, `with_n_ctx(Option<NonZeroU32>)`) 소스에 맞춘다. `Special::Plaintext`가 없으면 실제 변형 이름(예: `Special::Tokenize`의 반대 항목)을 쓴다.

- [ ] **Step 4: 게이트** — `mise exec -- cargo test -p babelay-engine --features metal`(첫 llama.cpp 빌드는 수 분; cmake는 mise가 제공), `mise exec -- cargo clippy -p babelay-engine --all-targets --features metal -- -D warnings`, `cargo fmt --all -- --check`, 그리고 기본 feature 빌드 `mise exec -- cargo test -p babelay-engine`도 통과. 실제 모델: `BABELAY_TEST_LLM="$HOME/Library/Application Support/com.babelay.app/models/llm/Qwen3.5-2B-Q4_K_M.gguf" mise exec -- cargo test -p babelay-engine --features metal translates_english_to_korean -- --ignored --nocapture` — 출력 문장과 소요 시간을 보고서에 적는다.
- [ ] **Step 5: Commit** — `feat(engine): local llama.cpp translator`

---

### Task 3: 클라우드 번역기 4종


**Files:**
- Create: `crates/babelay-engine/src/translate/cloud.rs`
- Modify: `crates/babelay-engine/Cargo.toml` (reqwest `json` feature; `serde_json` 을 일반 의존성으로), `crates/babelay-engine/src/translate/mod.rs` (`pub mod cloud;`)

**Interfaces (produces):**
```rust
pub struct OpenAiCompatible { pub base_url: String, pub api_key: String, pub model: String }   // POST {base_url}/chat/completions, Authorization: Bearer
pub struct Anthropic { pub base_url: String, pub api_key: String, pub model: String }          // POST {base_url}/v1/messages, headers x-api-key, anthropic-version: 2023-06-01; body {model, max_tokens: 512, system, messages:[{role:"user",content}]}
pub struct Gemini { pub base_url: String, pub api_key: String, pub model: String }             // POST {base_url}/v1beta/models/{model}:generateContent?key={key}; body {system_instruction:{parts:[{text}]}, contents:[{role:"user",parts:[{text}]}]}
pub struct DeepL { pub base_url: String, pub api_key: String }                                 // POST {base_url}/v2/translate, Authorization: DeepL-Auth-Key {key}; form text, target_lang(대문자 KO/EN/JA), source_lang(대문자, src가 알려진 경우)
impl Default for … { base_url = 실제 엔드포인트 }  // OpenAI "https://api.openai.com/v1", Anthropic "https://api.anthropic.com", Gemini "https://generativelanguage.googleapis.com", DeepL: 키가 ":fx" 로 끝나면 "https://api-free.deepl.com" 아니면 "https://api.deepl.com" (DeepL::new(api_key) 가 결정)
pub fn with_retry<T>(mut f: impl FnMut() -> Result<T, TranslateError>) -> Result<T, TranslateError>;   // RateLimited / Request(5xx) / Timeout 이면 200ms, 600ms 후 최대 2회 재시도; Auth/Empty 는 즉시 반환
pub(crate) fn map_status(status: u16, body: &str) -> TranslateError;   // 401|403 → Auth, 429 → RateLimited, 5xx → Request(format!("{status}: {body_head}")), else Request
```
공통: `reqwest::blocking::Client::builder().timeout(Duration::from_secs(10)).build()`; 연결/타임아웃 오류 → `Timeout` (reqwest `is_timeout()`) 또는 `Request`. 응답 텍스트는 `postprocess`; 빈 결과 → `Empty`. 프롬프트는 Task 1의 `system_prompt`/`user_prompt` 재사용(DeepL 제외). 각 `translate()`는 `with_retry(|| self.once(req))` 형태.

응답 파싱:
- OpenAI: `choices[0].message.content` (string)
- Anthropic: `content[0].text`
- Gemini: `candidates[0].content.parts[0].text`
- DeepL: `translations[0].text`

- [ ] **Step 1: 테스트(httpmock, 어댑터당 3개 = 12개)**

각 어댑터: (a) 요청 검증 + 정상 파싱 — `when.method(POST).path(...)` 에 헤더(`authorization`/`x-api-key`/`anthropic-version`)와 본문 일부(`body_contains("Korean")` 또는 DeepL `body_contains("target_lang=KO")`)를 매칭하고 위 JSON을 돌려줘 `"안녕"`을 얻는다; (b) 첫 응답 429, 두 번째 200 → 성공(`mock.hits() == 2`); (c) 401 → `Err(TranslateError::Auth)`. `with_retry` 단독 테스트 1개: 3회 연속 `RateLimited` 면 최종 `Err(RateLimited)` 이고 호출 횟수 3.

예:
```rust
#[test]
fn openai_parses_content_and_sends_bearer() {
    let s = MockServer::start();
    let m = s.mock(|w, t| {
        w.method(POST).path("/v1/chat/completions").header("authorization", "Bearer k").body_contains("Korean");
        t.status(200).json_body(serde_json::json!({"choices":[{"message":{"content":"안녕"}}]}));
    });
    let mut c = OpenAiCompatible { base_url: s.url("/v1"), api_key: "k".into(), model: "gpt-4o-mini".into() };
    let out = c.translate(&req("Hello", "en", "ko")).unwrap();
    assert_eq!(out, "안녕"); m.assert();
}
```

- [ ] **Step 2: 구현** — 위 사양대로. `serde_json::json!`로 본문 조립, `resp.json::<serde_json::Value>()`로 파싱 후 `pointer("/choices/0/message/content")` 등으로 추출.
- [ ] **Step 3: 게이트 + Commit** — `feat(engine): cloud translators (openai-compatible, anthropic, gemini, deepl)`

---

### Task 4: 파이프라인 번역 스레드와 `Translated` 이벤트


**Files:**
- Modify: `crates/babelay-engine/src/engine.rs`

**Interfaces (produces):**
- `EngineConfig` 필드 추가: `pub tgt_lang: Option<String>` (None = 번역 안 함).
- `EngineEvent::Translated { id: u64, text: String, lang: String }` (serde snake_case → `{"type":"translated", …}`), 그리고 번역 실패 `Error { code: "translate", message }`.
- `start(cfg, source, transcriber, translator: Option<Box<dyn Translator>>, gpu_active: bool, gpu_fallback: bool, tx)`; `start_default(cfg, translator: Option<Box<dyn Translator>>, tx)`.
- 규칙: 전사 스레드가 `Final{id,text,lang,…}`을 낸 직후 `(id, text, lang)`을 번역 큐(`sync_channel::<(u64,String,String)>(16)`, blocking send)에 넣는다(번역기가 없으면 큐 자체를 만들지 않는다). 번역 스레드는 `lang == tgt`면 건너뛰고(이벤트 없음), 아니면 직전 확정 원문 최대 2개(`VecDeque<String>`)를 `context`로 `translate` 호출 → `Translated{id, text, lang: tgt}`; 컨텍스트 큐에는 성공/실패와 무관하게 원문을 넣는다. 실패는 `Error{code:"translate", message}`를 발행하되 연속 실패 시 30초에 한 번만(마지막 발행 시각 기억), stderr에는 매번 로그.
- **`Stopped` 소유권:** 번역 스레드가 있으면 전사 스레드는 `Stopped`를 보내지 않고 번역 큐 송신단을 drop해 끝내고, 번역 스레드가 큐 종료 후 `Stopped`를 1회 보낸다. 번역 스레드가 없으면 기존처럼 전사 스레드가 보낸다. `EngineHandle::drain`은 번역 스레드도 join하며 패닉 백스톱(`Error{panic}` + `Stopped`)은 "마지막 스레드"의 join 실패에 대해 적용한다(두 스레드 중 하나라도 패닉했고 `Stopped`가 나가지 못했다면 1회 보낸다 — 간단히: 전사 join Err 이거나 번역 join Err 이면 백스톱 발행; 번역 스레드가 정상 종료했다면 그 스레드가 `Stopped`를 이미 보냈으므로 백스톱은 전사 join 결과를 무시한다).
- 번역 호출은 `catch_unwind(AssertUnwindSafe(..))`로 감싸 패닉을 `Error{code:"panic"}`로 바꾸고 루프를 유지한다(전사 스레드와 같은 패턴).

- [ ] **Step 1: 테스트** (기존 `FakeSource`/`FakeTranscriber`/`drain_until` 재사용)

```rust
struct UpperTranslator;
impl crate::translate::Translator for UpperTranslator {
    fn name(&self) -> &str { "upper" }
    fn translate(&mut self, req: &crate::translate::TranslateRequest) -> Result<String, crate::translate::TranslateError> { Ok(req.text.to_uppercase()) }
}
struct FailingTranslator;
impl crate::translate::Translator for FailingTranslator {
    fn name(&self) -> &str { "fail" }
    fn translate(&mut self, _: &crate::translate::TranslateRequest) -> Result<String, crate::translate::TranslateError> { Err(crate::translate::TranslateError::Request("boom".into())) }
}

#[test]
fn final_is_followed_by_translated_with_same_id_and_stopped_once() {
    // cfg.tgt_lang = Some("ko"); FakeTranscriber 는 lang "en" 을 돌려준다
    // 기대: Final{id:1} 뒤에 Translated{id:1, text == final.text.to_uppercase(), lang:"ko"}, 마지막이 Stopped 이고 Stopped 는 정확히 1개
}
#[test]
fn no_translation_when_source_equals_target() {
    // cfg.tgt_lang = Some("en") → Translated 이벤트 없음, Final 은 그대로, Stopped 1개
}
#[test]
fn failing_translator_emits_one_translate_error_and_keeps_finals() {
    // FailingTranslator → Error{code:"translate"} 가 정확히 1개(연속 실패 억제), Final 은 정상, Stopped 1개
}
#[test]
fn started_and_stopped_without_translator_unchanged() {
    // translator None → 기존 pipeline 테스트와 동일 동작(기존 테스트가 이미 있다면 start 시그니처만 갱신)
}
```

- [ ] **Step 2: 구현** — `translate_loop(rx, mut translator, tgt: String, tx)`; 컨텍스트 `VecDeque<String>` 길이 2; 실패 억제 타이머 `Option<Instant>`. `transcribe_loop`에 `Option<SyncSender<(u64,String,String)>>`와 `emit_stopped: bool` 인자 추가. `start`에서 채널·스레드 생성; `EngineHandle`에 `translator: Option<JoinHandle<()>>`.
- [ ] **Step 3: 게이트** — `mise exec -- cargo test -p babelay-engine --features metal engine`, 기본 feature, clippy, fmt.
- [ ] **Step 4: Commit** — `feat(engine): translation stage and Translated event`

---

### Task 5: Tauri — 키 저장, 번역기 조립, 연결 테스트, 세션·히스토리 연결


**Files:**
- Create: `src-tauri/src/keys.rs`, `src-tauri/src/translator.rs`
- Modify: `src-tauri/Cargo.toml` (`keyring = "4"`), `src-tauri/src/{lib.rs, commands.rs, session.rs, history.rs}`

**Interfaces (produces):**
- `keys.rs`: `pub fn set(provider: &str, key: &str) -> Result<(), String>` (빈 키면 `delete`), `pub fn get(provider: &str) -> Result<Option<String>, String>` (`NoEntry` → `Ok(None)`), `pub fn delete(provider: &str) -> Result<(), String>` (없어도 Ok), `pub fn has(provider: &str) -> bool`. `keyring::Entry::new("com.babelay.app", provider)`. provider 는 `openai|anthropic|gemini|deepl|custom` 중 하나만 허용(그 외 `Err("unknown_provider")`).
- `translator.rs`:
  - `pub fn resolve_tgt(settings: &Settings) -> String` — `overlay.subtitle_lang == "system"` → `crate::i18n::resolve("system")`의 코드(`Lang::Ko→"ko"` 등), 아니면 그대로.
  - `pub fn build(settings: &Settings, models_dir: &Path) -> Result<Option<(Box<dyn Translator>, bool /*gpu_fallback*/)>, String>` — `overlay.display_mode == "source"` → `Ok(None)`; `backend == "local"`: `find(local_model)` 없거나 `!installed` → `Err("translation_model_missing")`, `LocalLlm::load(path, settings.asr.gpu)` → `(Box::new(t), fell_back)`; `backend == "cloud"`: `keys::get(provider)?` 없으면 `Err("api_key_missing")`, `custom`이면 `base_url` 빈 문자열 → `Err("base_url_missing")`, 어댑터 생성(`openai` → `OpenAiCompatible{base_url:"https://api.openai.com/v1"}`, `custom` → base_url 설정값, `anthropic/gemini` → `..Default::default()` + key + model, `deepl` → `DeepL::new(key)`), 모델명이 빈 문자열이면 기본값(`gpt-4o-mini` / `claude-haiku-4-5-20251001` / `gemini-2.5-flash`).
  - `pub fn test_translation(settings: &Settings, models_dir: &Path) -> TestResult` — `build` 후 `TranslateRequest{text:"Good morning.", src:"en", tgt: resolve_tgt(..), context: vec![]}` 1회; `TestResult { ok: bool, ms: u64, text: String, error: Option<String> }` (Serialize). `None`(표시 모드 source)이면 `ok:false, error:Some("display_mode_source")`.
- 커맨드(`commands.rs`, 등록): `set_api_key(provider: String, key: String) -> Result<(), String>`, `has_api_key(provider: String) -> bool`, `delete_api_key(provider: String) -> Result<(), String>`, `#[tauri::command(async)] test_translation(app) -> Result<TestResult, String>` — `spawn_blocking`으로 실행, 20초 넘으면 `Err("timeout")`.
- `session.rs`: `start`의 동기 사전 검사에 번역기 조립 가능 여부를 **키/모델 존재 수준에서만** 확인(`translator::precheck(settings, models_dir) -> Result<(), String>`: `translation_model_missing` / `api_key_missing` / `base_url_missing`; 로드는 하지 않음). `run_session` 스레드에서 `translator::build`(무거운 로드) 후 `start_default(cfg{tgt_lang: Some(resolve_tgt) 또는 None(표시 모드 source)}, translator, tx)`; `build` 실패는 `engine-event Error{code:"start_failed", message}`로. `Translated` 이벤트를 `history::on_translated(&app, &ev)`로 넘긴다. `history::begin`의 `translator` 컬럼 값: `"local:<model_id>"` / `"cloud:<provider>/<model>"` / `NULL`(표시 모드 source).
- `history.rs`: 스키마에 `CREATE TRIGGER IF NOT EXISTS segments_au AFTER UPDATE ON segments BEGIN INSERT INTO segments_fts(segments_fts, rowid, src_text, tgt_text) VALUES('delete', old.id, old.src_text, old.tgt_text); INSERT INTO segments_fts(rowid, src_text, tgt_text) VALUES (new.id, new.src_text, new.tgt_text); END;` 추가; `begin_session(src, tgt, asr_model, translator: Option<&str>)`; `SessionState`에 `final_rows: Mutex<HashMap<u64, i64>>`(begin에서 clear, `on_final`에서 `insert_segment`가 돌려준 row id를 engine `id`로 기록 — `insert_segment`가 `Result<i64>`를 돌려주게 바꾼다); `pub fn update_translation(&self, row_id: i64, text: &str) -> rusqlite::Result<()>`; `pub fn on_translated(app, ev)` (id → row id 조회 후 UPDATE; 없으면 무시); export: SRT 블록은 `src_text` 줄 + (있으면) `tgt_text` 줄, TXT는 `src\ttgt`(tgt 없으면 `src`).
- 테스트: `history` 인메모리 테스트에 `translation_update_is_searchable_and_exported` 추가(세그먼트 삽입 → `update_translation(row, "안녕 세계")` → `search("세계")` 1건, `export(srt)`에 두 줄, `export(txt)`에 탭 구분). `keys.rs`는 `#[ignore]` 라운드트립 테스트 1개(키체인 프롬프트 가능). `translator::resolve_tgt` 단위 테스트 2개(고정값 / system).

- [ ] **Step 1: 테스트 → Step 2: 구현 → Step 3: 게이트(`mise exec -- cargo test --workspace`, clippy, fmt, `mise exec -- yarn tauri build --debug --no-bundle`)**
- [ ] **Step 4: Commit** — `feat: api keys in keyring, translator assembly, translation test command, history translations`

---

### Task 6: 프론트엔드 — 번역 표시, 오버레이 한 세트 규칙, 번역 탭


**Files:**
- Modify: `src/lib/{types.ts, tauri.ts, session.ts, overlay.ts, models.ts}`, `src/pages/{OverlayWindow.tsx, main/Live.tsx, main/History.tsx, settings/Translation.tsx}`, `src/locales/{ko,en,ja}.json`, `src/test/session.test.ts`
- Create: `src/test/overlay.test.ts`

**Interfaces / 규칙:**
- `types.ts`: `EngineEvent`에 `{ type: "translated"; id: number; text: string; lang: string }`; `Final`(session.ts)에 `tgt?: string`; `TestTranslationResult { ok: boolean; ms: number; text: string; error: string | null }`.
- `tauri.ts`: `setApiKey(provider, key)` → `invoke("set_api_key", { provider, key })`, `hasApiKey(provider)`, `deleteApiKey(provider)`, `testTranslation()`.
- `session.ts` reducer: `translated` → 같은 `id`의 final에 `tgt` 부착(없는 id는 무시), `lastEventAt` 갱신.
- `overlay.ts`: `pairForOverlay(finals: Final[], now: number, lastFinalAt: number, waitMs = 3000): { source: string; translated: string }` — 마지막 final에 `tgt`가 있으면 `{source: last.text, translated: last.tgt}`; 없고 `now - lastFinalAt < waitMs`이고 직전 final이 있으면 **직전 세트**(`prev.text`, `prev.tgt ?? ""`); 그 외(대기 초과 또는 직전 없음) `{source: last.text, translated: ""}`; finals 비어 있으면 둘 다 "". 오버레이 컴포넌트는 "번역 대기 중"(마지막 final에 tgt 없음 && 대기 시간 내)일 때만 100ms 간격 타이머로 재평가한다. 이후 `overlayLines(mode, source, partial, translated)` 기존 함수 그대로.
- Live: 각 final 아래 `tgt`를 굵게(있을 때); History: 세그먼트 `tgt_text` 줄과 검색 결과에도 표시.
- Translation 탭(클라우드 섹션): `SettingRow label={t("translation.apiKey")}` — `input type="password" className="input input-sm w-56"` + 저장 `btn btn-sm btn-primary`; 저장돼 있으면(`has_api_key`) `badge badge-neutral`로 `translation.saved` + 삭제 `btn btn-ghost btn-sm`; 프로바이더 변경/저장/삭제 후 `has_api_key` 재조회; 입력값은 저장 후 비운다(키를 상태에 오래 두지 않음). 로컬/클라우드 공통 하단에 `translation.test` 버튼(`btn btn-outline btn-sm`, 진행 중 `loading loading-spinner`) → 결과를 `alert alert-success` / `alert alert-error` 한 줄(`translation.testResult`: `{{ms}} ms · {{text}}`, 실패면 매핑된 오류 메시지)로 5초간 표시.
- `models.ts` `ERROR_KEYS` 추가: `translation_model_missing → errors.translationModelMissing`, `api_key_missing → errors.apiKeyMissing`, `base_url_missing → errors.baseUrlMissing`, `translate → errors.translateFailed`, `display_mode_source → errors.displayModeSource`, `unknown_provider → errors.unknownProvider`.
- 로케일(ko/en/ja): `translation.apiKey`("API 키"/"API key"/"API キー"), `translation.save`("저장"/"Save"/"保存"), `translation.saved`("저장됨 ●●●●"/"Saved ●●●●"/"保存済み ●●●●"), `translation.deleteKey`("삭제"/"Delete"/"削除"), `translation.test`("연결 테스트"/"Test connection"/"接続テスト"), `translation.testResult`("{{ms}} ms · {{text}}" ×3), `errors.translationModelMissing`("번역 모델이 설치되어 있지 않습니다"/"The translation model is not installed"/"翻訳モデルがインストールされていません"), `errors.apiKeyMissing`("API 키를 입력하세요"/"Enter an API key"/"API キーを入力してください"), `errors.baseUrlMissing`("Base URL을 입력하세요"/"Enter a base URL"/"Base URL を入力してください"), `errors.translateFailed`("번역에 실패했습니다"/"Translation failed"/"翻訳に失敗しました"), `errors.displayModeSource`("표시 모드가 원문만이라 번역하지 않습니다"/"Display mode is source only, so nothing is translated"/"表示モードが原文のみのため翻訳しません"), `errors.unknownProvider`("알 수 없는 프로바이더"/"Unknown provider"/"不明なプロバイダー"). 고아 키 스윕(사용 안 하는 키 삭제; 템플릿 키 접두어는 유지).
- 테스트: `session.test.ts` — `translated`가 같은 id에 `tgt`를 붙이고 없는 id는 무시; `overlay.test.ts` — `pairForOverlay` 3케이스(번역 있음 / 대기 중이면 직전 세트 유지 / 3초 초과면 원문만) + finals 비어 있음.

- [ ] **Step 1: 테스트 → Step 2: 구현 → Step 3: 게이트(`mise exec -- yarn tsc --noEmit`, `mise exec -- yarn test`, `mise exec -- yarn build`)**
- [ ] **Step 4: Commit** — `feat(ui): translations in overlay/live/history, api key and connection test`

---

### Task 7: 문서


- `docs/superpowers/specs/2026-09-02-babelay-design.md`: §6에 실제 어댑터 엔드포인트(OpenAI `/v1/chat/completions`, Anthropic `/v1/messages`, Gemini `generateContent`, DeepL `/v2/translate` free/pro 자동 선택), 재시도(429/5xx/타임아웃 2회, 200/600ms), 로컬 LLM 규칙(요청마다 새 컨텍스트, greedy, Qwen3 `/no_think`, `<think>` 제거, 최대 토큰 3배), 키 저장(keyring service `com.babelay.app`, user = provider); §7.4의 "3단계 구현 항목" 표현을 완료로; §8에 UPDATE 트리거 추가됨(`segments_au`), `translator` 컬럼 값 형식; §11 item 3 "— 완료(2026-09-03)".
- `README.md`: API 키 저장 위치(Keychain / Credential Manager), `BABELAY_TEST_LLM` 무시 테스트 실행 명령, 번역 연결 테스트 사용법 한 줄.
- Create `docs/superpowers/2026-09-03-phase3-gui-checklist.md` (한국어, 10항목): 로컬 모델(Qwen 3.5 2B) 번역이 오버레이에 원문+번역 한 세트로 표시(3초 대기 규칙 관찰), 표시 모드 3종, 원어==타겟이면 번역 없음, 클라우드 키 저장 → "저장됨 ●●●●" → 연결 테스트 ms 표시 → 삭제, 잘못된 키로 오류 배너 1회, 히스토리 상세·검색에 번역 표시, SRT 두 줄 내보내기, GPU 토글 off 시 로컬 번역 CPU 동작, 캡처 중 설정 변경은 다음 세션부터 적용.
- Commit — `docs: phase 3 spec updates and GUI checklist`

---

