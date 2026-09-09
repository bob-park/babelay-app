//! 로컬 번역 LLM 캐시. 스펙 §4.3: 첫 번역 시점에 로드하고, 모델(경로 또는 GPU 토글)이
//! 바뀌기 전까지 세션이 끝나도 프로세스에 남는다 — 캡처 시작이 1.3GB 로드를 기다리지 않고,
//! stop → start 나 연결 테스트가 같은 모델을 다시 읽지 않는다.
//! GPU 로드가 실패해 CPU 로 내려갔으면 `EngineEvent::CpuFallback{stage:"translate"}` 를
//! `engine-event` 로 낸다(4단계 스펙 §5) — 세션(=`SharedLlm` 인스턴스)마다 한 번.
use babelay_engine::engine::EngineEvent;
use babelay_engine::translate::local::LocalLlm;
use babelay_engine::translate::{TranslateError, TranslateRequest, Translator};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{AppHandle, Emitter, Manager};

struct Loaded {
    path: PathBuf,
    gpu: bool,
    /// GPU 로드 실패 후 CPU 로 올라온 모델인지. 캐시된 채 다음 세션이 써도 알려야 한다.
    fell_back: bool,
    llm: LocalLlm,
}

/// 프로세스 전역 캐시(`app.manage`). 담긴 모델은 최대 하나다.
/// `app` 이 없으면(테스트의 `Default`) 폴백 이벤트를 내지 않는다.
#[derive(Default, Clone)]
pub struct LlmCache {
    slot: Arc<Mutex<Option<Loaded>>>,
    app: Option<AppHandle>,
}

impl LlmCache {
    pub fn new(app: AppHandle) -> Self {
        Self {
            slot: Arc::default(),
            app: Some(app),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<Loaded>> {
        self.slot.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// 담긴 모델을 내린다. 종료 경로에서 부른다 — Metal 버퍼를 쥔 llama 컨텍스트가 살아 있는 채로
    /// `exit()` 가 돌면 ggml 의 정적 소멸자가 `GGML_ASSERT([rsets->data count] == 0)` 로 abort 한다.
    pub fn clear(&self) {
        *self.lock() = None;
    }

    /// 모델이 들어 있는지. 테스트용.
    #[cfg(test)]
    fn is_loaded(&self) -> bool {
        self.lock().is_some()
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
    cache: LlmCache,
    path: PathBuf,
    gpu: bool,
    /// 폴백 이벤트는 인스턴스마다 한 번.
    notified: bool,
}

impl SharedLlm {
    pub fn new(cache: LlmCache, path: PathBuf, gpu: bool) -> Self {
        Self {
            cache,
            path,
            gpu,
            notified: false,
        }
    }
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
                fell_back,
                llm,
            });
        }
        // 방금 채웠거나 이미 맞는 모델이 들어 있다.
        let l = g
            .as_mut()
            .ok_or_else(|| TranslateError::Load("llm cache empty".into()))?;
        if l.fell_back && !self.notified {
            self.notified = true;
            if let Some(app) = &self.cache.app {
                let _ = app.emit(
                    "engine-event",
                    EngineEvent::CpuFallback {
                        stage: "translate".into(),
                    },
                );
            }
        }
        l.llm.translate(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_empties_the_slot() {
        let c = LlmCache::default();
        assert!(!c.is_loaded());
        c.clear();
        assert!(!c.is_loaded());
        // 모델 없이 슬롯이 찬 상태를 만들 수 없으니 clear 가 잠금을 오염시키지 않는 것까지만 본다.
        let c2 = c.clone();
        c2.clear();
        assert!(!c.is_loaded());
    }

    #[test]
    #[ignore = "needs BABELAY_TEST_LLM=<gguf>; loads a real model, clears, then checks the slot is empty"]
    fn clear_drops_a_loaded_model() {
        let path = PathBuf::from(std::env::var("BABELAY_TEST_LLM").unwrap());
        let c = LlmCache::default();
        let mut t = SharedLlm::new(c.clone(), path, true);
        let req = TranslateRequest {
            text: "Good morning.".into(),
            src: "en".into(),
            tgt: "ko".into(),
            context: vec![],
        };
        t.translate(&req).unwrap();
        assert!(c.is_loaded());
        c.clear();
        assert!(!c.is_loaded());
        // 여기서 프로세스가 exit 해도 abort 하지 않아야 한다(수동 확인: 앱 종료).
    }
}
