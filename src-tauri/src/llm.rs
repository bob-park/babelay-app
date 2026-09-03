//! 로컬 번역 LLM 캐시. 스펙 §4.3: 첫 번역 시점에 로드하고, 모델(경로 또는 GPU 토글)이
//! 바뀌기 전까지 세션이 끝나도 프로세스에 남는다 — 캡처 시작이 1.3GB 로드를 기다리지 않고,
//! stop → start 나 연결 테스트가 같은 모델을 다시 읽지 않는다.
use babelay_engine::translate::local::LocalLlm;
use babelay_engine::translate::{TranslateError, TranslateRequest, Translator};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Manager};

struct Loaded {
    path: PathBuf,
    gpu: bool,
    llm: LocalLlm,
}

/// 프로세스 전역 캐시(`app.manage`). 담긴 모델은 최대 하나다.
#[derive(Default, Clone)]
pub struct LlmCache(Arc<Mutex<Option<Loaded>>>);

impl LlmCache {
    fn lock(&self) -> MutexGuard<'_, Option<Loaded>> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

pub fn cache(app: &AppHandle) -> LlmCache {
    app.state::<LlmCache>().inner().clone()
}

/// 이 경로의 모델이 캐시에 있으면 내린다. 파일을 지우기 전에 불러야 한다 —
/// Windows 는 mmap 된 파일을 지우지 못한다.
pub fn evict(app: &AppHandle, path: &Path) {
    let c = cache(app);
    let mut g = c.lock();
    if g.as_ref().is_some_and(|l| l.path == path) {
        *g = None;
    }
}

/// 캐시를 공유하는 번역기. 첫 `translate` 에서 로드하고, 경로나 GPU 설정이 다르면 갈아 끼운다.
pub struct SharedLlm {
    pub cache: LlmCache,
    pub path: PathBuf,
    pub gpu: bool,
}

impl Translator for SharedLlm {
    fn name(&self) -> &str {
        "local"
    }

    // ponytail: 번역 내내 캐시 잠금을 쥔다. 번역 워커는 세션당 하나뿐이라 경합이 없고,
    // 겹치는 호출(연결 테스트)은 줄 세우는 편이 두 번 로드하는 것보다 낫다.
    fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError> {
        let mut g = self.cache.lock();
        if !matches!(&*g, Some(l) if l.path == self.path && l.gpu == self.gpu) {
            // 먼저 비운다 — 새 모델을 올리는 동안 옛 모델이 메모리를 두 배로 쓰지 않게.
            *g = None;
            let (llm, fell_back) = LocalLlm::load(&self.path, self.gpu)?;
            if fell_back {
                eprintln!("babelay: 번역 모델 GPU 로드 실패 — CPU 로 폴백");
            }
            *g = Some(Loaded {
                path: self.path.clone(),
                gpu: self.gpu,
                llm,
            });
        }
        // 방금 채웠거나 이미 맞는 모델이 들어 있다.
        match g.as_mut() {
            Some(l) => l.llm.translate(req),
            None => Err(TranslateError::Load("llm cache empty".into())),
        }
    }
}
