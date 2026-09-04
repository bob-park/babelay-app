# Babelay 4단계: 패스쓰루 안정화 + 장치 변경 자가 복구 + 잔여 백로그 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 원어 == 타겟이면 Whisper 오감지에 흔들리지 않고 원문을 바로 보여주고, 세션 중 기본 출력 장치가 바뀌어도 캡처가 이어지며, 로컬 LLM 의 CPU 폴백을 Live 배지로 알리고, API 키 변경 버튼과 비동기 연결 테스트를 넣는다.

**Architecture:** 엔진 `transcribe_loop` 가 최근 Final 3개의 감지 언어 다수결로 `Final.lang` 을 확정한다(프론트·히스토리는 이미 `Final.lang` 을 본다). 장치 변경은 캡처 모듈이 스스로 복구한다 — macOS 는 `tap.m` 의 기본 출력 장치 리스너가 집계 장치+IOProc 만 재생성하고, Windows 는 읽기 루프가 1초마다 기본 장치 id 를 비교해 재연결한다. 엔진 청커는 프레임 포맷이 바뀌면 리샘플러를 새로 만든다. CPU 폴백은 `LlmCache` 가 `AppHandle` 을 들고 `EngineEvent::CpuFallback` 을 `engine-event` 로 내며, 프론트 리듀서가 기존 `gpuFallback` 배지를 켠다.

**Tech Stack:** Rust(babelay-engine, Tauri 2 src-tauri), Core Audio(ObjC 심 `csrc/tap.m`), wasapi 0.24, React 19 + zustand 5 + vitest 4.

**Spec:** `docs/superpowers/specs/2026-09-04-phase4-passthrough-device-design.md` (§3 언어 판정, §4 장치 변경, §5 폴백 표시, §6 키 변경, §7 연결 테스트, §8 테스트, §9 문서). 상위 스펙 `docs/superpowers/specs/2026-09-02-babelay-design.md`.

## Global Constraints

- 브랜치 `main` 에서 작업(사용자 지시). 셸에 mise 가 활성화되어 있지 않으면 `mise exec -- cargo/yarn …`.
- 게이트: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `yarn tsc --noEmit`, `yarn test`, `yarn build`.
- `babelay-engine` 은 Tauri 에 의존하지 않는다. 엔진의 유일한 출력은 `EngineEvent` 채널. `AudioSource` 트레이트는 바꾸지 않는다.
- 새 의존성 없음. `tauri::async_runtime` 은 tauri 재수출.
- 오버레이 "한 세트 동시 교체" 규칙(상위 스펙 §7.4)과 `TRANSLATION_WAIT_MS = 3000` 은 그대로.
- 세 로케일(`src/locales/{ko,en,ja}.json`) 키 집합 동일(`src/test/locales.test.ts` 가 검사).
- Windows 코드는 이 머신에서 실행할 수 없다. 격리 크레이트 `cargo check --target x86_64-pc-windows-msvc` 까지가 게이트(Task 8 참고).
- 커밋 접두어 `feat:`/`fix:`/`test:`/`docs:`. 트레일러:
  ```
  Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01BgsDcmiqzXdg2uD5iaxZjq
  ```
- ponytail: 새 추상화 없음. 한계를 두는 단순화에는 `// ponytail:` 주석으로 상한과 업그레이드 경로를 적는다.

## 파일 구조

| 파일 | 책임 | 태스크 |
|---|---|---|
| `crates/babelay-engine/src/engine.rs` | `LangVote`(다수결), `transcribe_loop` 에서 사용, `chunker_loop` 리샘플러 교체, `EngineEvent::CpuFallback` | 1, 2, 4 |
| `crates/babelay-engine/src/audio.rs` | `Resampler::format()` 접근자 | 2 |
| `src-tauri/src/translator.rs` | `target(settings) -> Option<String>` | 3 |
| `src-tauri/src/session.rs` | `tgt_lang: translator::target(&settings)` | 3 |
| `src-tauri/src/llm.rs` | `LlmCache{slot, app}`, `Loaded.fell_back`, `SharedLlm::new`, 폴백 이벤트 1회 | 4 |
| `src-tauri/src/lib.rs` | `LlmCache::new(app.handle().clone())` | 4 |
| `src/lib/types.ts`, `src/lib/session.ts`, `src/test/session.test.ts` | `cpu_fallback` 이벤트·리듀서 | 4 |
| `src-tauri/src/commands.rs` | `test_translation` async | 5 |
| `src/pages/settings/Translation.tsx`, `src/locales/*.json` | 키 `변경` 버튼 | 6 |
| `crates/babelay-engine/csrc/tap.m` | 기본 출력 리스너 + 집계 장치 재생성 | 7 |
| `crates/babelay-engine/src/capture/windows.rs` | 기본 장치 폴링 + 재연결 | 8 |
| 상위 스펙 §4.1/§4.2/§4.3/§4.4/§11, `docs/superpowers/2026-09-04-phase4-gui-checklist.md` | 문서 | 9 |

---

### Task 1: Final 언어 다수결 (`LangVote`)

**Files:**
- Modify: `crates/babelay-engine/src/engine.rs` (상수 블록 뒤에 `LangVote`; `transcribe_loop` Final 분기; 테스트 모듈)

**Interfaces:**
- Produces: `struct LangVote` (crate-private), `LangVote::new() -> Self`, `LangVote::push(&mut self, detected: String) -> String`. `EngineEvent::Final.lang` 과 번역 큐 `(id, text, lang)` 의 `lang` 은 확정 언어.

- [ ] **Step 1: 실패하는 테스트**

`engine.rs` 의 `mod tests` 끝에 추가:

```rust
    #[test]
    fn lang_vote_majority_overrides_a_single_misdetection() {
        let mut v = LangVote::new();
        assert_eq!(v.push("en".into()), "en", "첫 Final 은 감지값 그대로");
        assert_eq!(v.push("en".into()), "en");
        assert_eq!(v.push("cy".into()), "en", "[en, en, cy] 다수결은 en");
    }

    #[test]
    fn lang_vote_tie_keeps_the_current_detection() {
        let mut v = LangVote::new();
        v.push("en".into());
        assert_eq!(v.push("ko".into()), "ko", "[en, ko] 동률이면 이번 값");
        assert_eq!(v.push("ja".into()), "ja", "[en, ko, ja] 모두 1표면 이번 값");
    }

    #[test]
    fn lang_vote_follows_a_real_switch_after_two_finals() {
        let mut v = LangVote::new();
        v.push("en".into());
        v.push("en".into());
        assert_eq!(v.push("ko".into()), "en", "첫 전환 Final 은 아직 en");
        assert_eq!(v.push("ko".into()), "ko", "[en, ko, ko] 부터 ko");
    }
```

- [ ] **Step 2: 실패 확인**

Run: `mise exec -- cargo test -p babelay-engine lang_vote`
Expected: 컴파일 오류 `cannot find struct, variant or union type LangVote`.

- [ ] **Step 3: 구현**

`TRANSLATE_CONTEXT` 상수 아래에:

```rust
/// Final 언어 다수결 창(스펙 4단계 §3.1). Whisper 는 Final 마다 언어를 새로 감지하고 짧은
/// 발화에서 자주 틀린다 — 한 번의 오감지가 번역 호출(또는 패스쓰루 누락)로 이어지지 않게 한다.
const LANG_WINDOW: usize = 3;

/// 최근 `LANG_WINDOW` 개 Final 의 감지 언어 다수결. 동률이면 이번 감지값.
/// ponytail: 언어가 세션 중 실제로 바뀌면 Final 2개만큼 늦게 따라간다. 더 빨리 따라가야 하면 창을 줄인다.
struct LangVote(VecDeque<String>);

impl LangVote {
    fn new() -> Self {
        Self(VecDeque::with_capacity(LANG_WINDOW + 1))
    }

    /// 이번 감지값을 넣고 확정 언어를 돌려준다.
    fn push(&mut self, detected: String) -> String {
        self.0.push_back(detected.clone());
        if self.0.len() > LANG_WINDOW {
            self.0.pop_front();
        }
        let count = |l: &str| self.0.iter().filter(|x| x.as_str() == l).count();
        let mine = count(&detected);
        self.0
            .iter()
            .find(|l| count(l) > mine)
            .cloned()
            .unwrap_or(detected)
    }
}
```

`transcribe_loop` 에서 `let mut lagging = false;` 아래에 `let mut vote = LangVote::new();` 를 넣고, Final 분기를 다음으로 바꾼다:

```rust
                Ok(Ok(segs)) => {
                    if let Some(s) = segs.into_iter().next() {
                        let id = next_id;
                        next_id += 1;
                        // 감지값 하나가 아니라 최근 Final 들의 다수결로 원어를 정한다(오감지 완화).
                        let lang = vote.push(s.lang);
                        let job = translate_tx
                            .as_ref()
                            .map(|_| (id, s.text.clone(), lang.clone()));
                        // 원문이 먼저다. 번역 큐 사정이 자막을 늦추면 안 된다.
                        let _ = tx.send(EngineEvent::Final {
                            id,
                            text: s.text,
                            lang,
                            start_ms,
                            end_ms,
                        });
                        // 큐가 차면 그 조각은 버린다 — 원문만 보이고 전사는 계속 흐른다.
                        if let (Some(q), Some(job)) = (&translate_tx, job) {
                            let _ = q.try_send(job);
                        }
                    }
                }
```

- [ ] **Step 4: 통과 확인**

Run: `mise exec -- cargo test -p babelay-engine`
Expected: `lang_vote_*` 3개 포함 전부 PASS. 기존 파이프라인 테스트(`no_translation_when_source_equals_target` 등)는 Final 하나만 내므로 영향 없음.

- [ ] **Step 5: Commit**

```bash
git add crates/babelay-engine/src/engine.rs
git commit -m "feat(engine): decide a Final's language by majority of the last three detections"
```

---

### Task 2: 프레임 포맷 변경 시 리샘플러 교체

**Files:**
- Modify: `crates/babelay-engine/src/audio.rs` (`impl Resampler` 에 `format()`)
- Modify: `crates/babelay-engine/src/engine.rs` (`chunker_loop`; 테스트)

**Interfaces:**
- Produces: `Resampler::format(&self) -> (u32, u16)` (src_rate, channels).
- Consumes: `Frame { samples, rate, channels }`, `Job { ev, enqueued }`, `ChunkEvent::Final { pcm, .. }`.

- [ ] **Step 1: 실패하는 테스트**

`engine.rs` `mod tests` 에 추가:

```rust
    /// 장치가 바뀌면 프레임 포맷도 바뀐다. 리샘플러가 첫 프레임에 고정돼 있으면 두 번째 구간의
    /// 샘플 수가 틀려진다(44.1k 모노를 48k 스테레오로 읽으면 약 1/4 로 줄어든다).
    #[test]
    fn chunker_follows_a_frame_format_change() {
        let (ftx, frx) = mpsc::channel::<Frame>();
        let (ctx, crx) = mpsc::sync_channel::<Job>(8);
        let h = std::thread::spawn(move || chunker_loop(frx, ctx));
        let tone = |rate: u32, ch: u16| -> Vec<f32> {
            (0..rate as usize * ch as usize)
                .map(|i| 0.3 * ((i / ch as usize) as f32 * 0.1).sin())
                .collect()
        };
        ftx.send(Frame { samples: tone(48_000, 2), rate: 48_000, channels: 2 }).unwrap();
        ftx.send(Frame { samples: tone(44_100, 1), rate: 44_100, channels: 1 }).unwrap();
        drop(ftx);
        h.join().unwrap();
        let total: usize = crx
            .iter()
            .filter_map(|j| match j.ev {
                ChunkEvent::Final { pcm, .. } => Some(pcm.len()),
                _ => None,
            })
            .sum();
        // 1초 + 1초의 말소리 → 16k 에서 약 32000 샘플.
        assert!((30_000..=34_000).contains(&total), "final pcm samples = {total}");
    }
```

- [ ] **Step 2: 실패 확인**

Run: `mise exec -- cargo test -p babelay-engine chunker_follows`
Expected: FAIL, `final pcm samples = 23xxx` 근처(두 번째 구간이 1/4 로 줄어든다).

- [ ] **Step 3: 구현**

`audio.rs` `impl Resampler` 에:

```rust
    /// 이 리샘플러가 가정하는 입력 포맷 (rate, channels). 청커가 프레임 포맷 변화를 감지하는 데 쓴다.
    pub fn format(&self) -> (u32, u16) {
        (self.src_rate, self.channels)
    }
```

`engine.rs` `chunker_loop` 의 `let r = resampler.get_or_insert_with(...)` 를:

```rust
        // 장치가 바뀌면 rate/channels 도 바뀔 수 있다. 첫 프레임에 고정하지 않고 갈아 끼운다
        // (직전 보간 샘플 하나를 잃는다 — 전환 순간의 수 ms, 무시).
        if resampler
            .as_ref()
            .is_none_or(|r| r.format() != (f.rate, f.channels))
        {
            resampler = Some(Resampler::new(f.rate, f.channels));
        }
        let r = resampler.as_mut().expect("just set");
```

주의: `Resampler::new` 는 `channels.max(1)` 로 저장하므로 `format()` 의 channels 도 1 이상이다. `chunker_loop` 는 `f.channels == 0` 프레임을 그 전에 버리므로 비교가 어긋나지 않는다.

- [ ] **Step 4: 통과 확인**

Run: `mise exec -- cargo test -p babelay-engine`
Expected: 전부 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/babelay-engine/src/audio.rs crates/babelay-engine/src/engine.rs
git commit -m "fix(engine): rebuild the resampler when the frame format changes"
```

---

### Task 3: 원어 고정 == 타겟이면 번역 단계 생략

**Files:**
- Modify: `src-tauri/src/translator.rs` (`target()` + 테스트)
- Modify: `src-tauri/src/session.rs:82` (`tgt_lang`)

**Interfaces:**
- Produces: `pub fn target(settings: &Settings) -> Option<String>`.
- Consumes: `enabled(settings)`, `resolve_tgt(settings)`, `settings.asr.source_lang: String` (`"auto" | "ko" | "en" | "ja"`).

- [ ] **Step 1: 실패하는 테스트**

`translator.rs` `mod tests` 에:

```rust
    #[test]
    fn target_is_none_when_fixed_source_equals_target() {
        let mut s = Settings::default();
        s.overlay.subtitle_lang = "en".into();
        s.asr.source_lang = "en".into();
        assert_eq!(target(&s), None, "en→en 은 번역 단계를 만들지 않는다");
        s.asr.source_lang = "auto".into();
        assert_eq!(target(&s).as_deref(), Some("en"), "auto 는 항상 번역 단계");
        s.asr.source_lang = "ko".into();
        assert_eq!(target(&s).as_deref(), Some("en"));
        s.overlay.display_mode = "source".into();
        assert_eq!(target(&s), None, "원문만 모드는 번역 없음");
    }
```

- [ ] **Step 2: 실패 확인**

Run: `mise exec -- cargo test -p babelay target_is_none`
Expected: 컴파일 오류 `cannot find function target`.

- [ ] **Step 3: 구현**

`translator.rs` 의 `enabled` 아래에:

```rust
/// 엔진에 넘길 번역 타겟. 번역이 꺼져 있거나, 원어가 고정돼 있고 타겟과 같으면 `None` —
/// 번역 단계를 만들지 않으므로 `Started.target_lang` 도 null 이고 오버레이가 기다리지 않는다.
pub fn target(settings: &Settings) -> Option<String> {
    if !enabled(settings) {
        return None;
    }
    let tgt = resolve_tgt(settings);
    (settings.asr.source_lang != tgt).then_some(tgt)
}
```

`session.rs` `start` 의 `EngineConfig` 에서:

```rust
        tgt_lang: translator::target(&settings),
```

- [ ] **Step 4: 통과 확인**

Run: `mise exec -- cargo test --workspace`
Expected: 전부 PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/translator.rs src-tauri/src/session.rs
git commit -m "feat(app): skip the translation stage when a fixed source language equals the target"
```

---

### Task 4: 로컬 LLM CPU 폴백 이벤트 + Live 배지

**Files:**
- Modify: `crates/babelay-engine/src/engine.rs` (`EngineEvent::CpuFallback`)
- Modify: `src-tauri/src/llm.rs` (`LlmCache{slot, app}`, `Loaded.fell_back`, `SharedLlm::new`, 이벤트)
- Modify: `src-tauri/src/translator.rs` (`SharedLlm::new` 사용)
- Modify: `src-tauri/src/lib.rs:27` (`LlmCache::new`)
- Modify: `src/lib/types.ts`, `src/lib/session.ts`
- Test: `src/test/session.test.ts`

**Interfaces:**
- Produces: `EngineEvent::CpuFallback { stage: String }` (serde: `{ "type": "cpu_fallback", "stage": "translate" }`); `LlmCache::new(app: AppHandle) -> LlmCache`(`Default` 유지, app 없음 = 이벤트 안 냄); `SharedLlm::new(cache: LlmCache, path: PathBuf, gpu: bool) -> SharedLlm`.
- Consumes: `LocalLlm::load(path, gpu) -> Result<(LocalLlm, bool), TranslateError>`; 프론트 `SessionView.gpuFallback`.

- [ ] **Step 1: 실패하는 프론트 테스트**

`src/test/session.test.ts` 의 `describe` 안에:

```ts
  it("cpu_fallback turns on the badge only while capturing", () => {
    const idle = reduce(initialView, { type: "cpu_fallback", stage: "translate" });
    expect(idle.gpuFallback).toBe(false);
    let v = reduce(initialView, { type: "started", gpu_active: true, gpu_fallback: false, model_id: "m", source_lang: null, target_lang: "ko" });
    v = reduce(v, { type: "cpu_fallback", stage: "translate" });
    expect(v.gpuFallback).toBe(true);
    v = reduce(v, { type: "stopped" });
    expect(v.gpuFallback).toBe(false);
  });
```

- [ ] **Step 2: 실패 확인**

Run: `mise exec -- yarn tsc --noEmit`
Expected: `session.test.ts` 에서 `type: "cpu_fallback"` 이 `EngineEvent` 에 없다는 오류.

- [ ] **Step 3: 엔진 이벤트**

`engine.rs` `EngineEvent` 의 `Lagging` 앞에:

```rust
    /// GPU 로드가 실패해 CPU 로 내려갔다. `stage` 는 `"translate"`(로컬 LLM, 첫 번역 시점).
    /// Whisper 폴백은 `Started.gpu_fallback` 으로 나간다.
    CpuFallback {
        stage: String,
    },
```

`src-tauri/src/history.rs` 의 `on_event` 는 `_ => {}` 팔이 있어 컴파일된다. `cargo check --workspace` 로 확인.

- [ ] **Step 4: `llm.rs`**

파일 전체를 다음으로 교체(문서 주석·`evict` 유지):

```rust
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
```

`translator.rs` `build` 의 `Ok(Some(Box::new(SharedLlm { cache: cache.clone(), path, gpu: settings.asr.gpu })))` 를:

```rust
    Ok(Some(Box::new(SharedLlm::new(
        cache.clone(),
        path,
        settings.asr.gpu,
    ))))
```

`lib.rs` 의 `app.manage(llm::LlmCache::default());` 를:

```rust
            app.manage(llm::LlmCache::new(app.handle().clone()));
```

- [ ] **Step 5: 프론트**

`src/lib/types.ts` `EngineEvent` 유니언의 `lagging` 앞에:

```ts
  | { type: "cpu_fallback"; stage: string }
```

`src/lib/session.ts` `reduce` 의 `case "lagging":` 앞에:

```ts
    case "cpu_fallback":
      // 로컬 LLM 이 CPU 로 내려갔다. 연결 테스트도 이 이벤트를 내므로 캡처 중일 때만 배지를 켠다.
      return v.capturing ? { ...next, gpuFallback: true } : next;
```

- [ ] **Step 6: 게이트**

Run: `mise exec -- cargo test --workspace && mise exec -- cargo clippy --workspace --all-targets -- -D warnings && mise exec -- yarn tsc --noEmit && mise exec -- yarn test`
Expected: 전부 PASS(새 vitest 1개 포함).

- [ ] **Step 7: Commit**

```bash
git add crates/babelay-engine/src/engine.rs src-tauri/src/llm.rs src-tauri/src/translator.rs src-tauri/src/lib.rs src/lib/types.ts src/lib/session.ts src/test/session.test.ts
git commit -m "feat: report the local LLM's CPU fallback on the live badge"
```

---

### Task 5: 연결 테스트 비동기화 (M7)

**Files:**
- Modify: `src-tauri/src/commands.rs:190-204`

**Interfaces:**
- Produces: `pub async fn test_translation(app: AppHandle) -> Result<TestResult, String>` (커맨드 이름·반환 JSON 동일, 프론트 변경 없음).

- [ ] **Step 1: 구현**

```rust
/// 설정 그대로 한 문장을 번역해 본다. 로컬 LLM 로드가 수 초 걸리므로 워커 스레드에서 돌리고,
/// 기다리는 쪽도 `spawn_blocking` 으로 보내 런타임 워커를 잡지 않는다. 상한을 넘기면 워커는
/// 버린다(끝나면 스스로 사라진다).
#[tauri::command]
pub async fn test_translation(app: AppHandle) -> Result<crate::translator::TestResult, String> {
    let settings = app.state::<SettingsState>().get();
    let dir = crate::models::models_dir(&app)?;
    let cache = crate::llm::cache(&app);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(crate::translator::test_translation(&settings, &dir, &cache));
    });
    tauri::async_runtime::spawn_blocking(move || {
        rx.recv_timeout(TEST_TIMEOUT)
            .map_err(|_| "timeout".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 2: 게이트**

Run: `mise exec -- cargo clippy --workspace --all-targets -- -D warnings && mise exec -- cargo build -p babelay`
Expected: 오류 없음. `generate_handler!` 는 async 커맨드를 그대로 받는다.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "fix(app): run the translation test off the async runtime workers"
```

---

### Task 6: API 키 '변경' 버튼 (M11)

**Files:**
- Modify: `src/pages/settings/Translation.tsx`
- Modify: `src/locales/ko.json`, `src/locales/en.json`, `src/locales/ja.json` (`translation.changeKey`)

**Interfaces:**
- Consumes: `api.setApiKey(provider, key)`, `api.hasApiKey(provider)`, `api.deleteApiKey(provider)`, 로케일 `common.cancel`(이미 있음).

- [ ] **Step 1: 로케일**

세 파일의 `"translation"` 객체에 `"deleteKey"` 옆에 키 추가:

- ko: `"changeKey": "변경"`
- en: `"changeKey": "Change"`
- ja: `"changeKey": "変更"`

- [ ] **Step 2: 컴포넌트**

`Translation.tsx`:

`const [saved, setSaved] = useState(false);` 아래에:

```tsx
  // 저장된 키를 새 값으로 덮어쓰는 중. 프로바이더가 바뀌면 접는다.
  const [editing, setEditing] = useState(false);
```

프로바이더 `useEffect` 의 `setKey("");` 뒤에 `setEditing(false);`.

`saveKey` 를:

```tsx
  const saveKey = () => {
    api.setApiKey(provider, key)
      .then(() => { setKey(""); setEditing(false); return api.hasApiKey(provider); })
      .then(setSaved)
      .catch(report);
  };
```

API 키 `SettingRow` 를:

```tsx
          <SettingRow as="div" label={t("translation.apiKey")}>
            {saved && !editing ? (
              <>
                <span className="badge badge-neutral">{t("translation.saved")}</span>
                <button type="button" className="btn btn-ghost btn-sm" onClick={() => setEditing(true)}>{t("translation.changeKey")}</button>
                <button type="button" className="btn btn-ghost btn-sm" onClick={deleteKey}>{t("translation.deleteKey")}</button>
              </>
            ) : (
              <>
                <input
                  type="password"
                  autoComplete="off"
                  aria-label={t("translation.apiKey")}
                  className={input}
                  value={key}
                  onChange={(e) => setKey(e.target.value)}
                  onKeyDown={(e) => { if (e.key === "Enter" && key.trim()) saveKey(); }}
                />
                <button type="button" className="btn btn-sm btn-primary" disabled={!key.trim()} onClick={saveKey}>{t("translation.save")}</button>
                {editing && <button type="button" className="btn btn-ghost btn-sm" onClick={() => { setKey(""); setEditing(false); }}>{t("common.cancel")}</button>}
              </>
            )}
          </SettingRow>
```

- [ ] **Step 3: 게이트**

Run: `mise exec -- yarn tsc --noEmit && mise exec -- yarn test && mise exec -- yarn build`
Expected: PASS. `locales.test.ts` 가 세 로케일 키 일치를 확인한다. 컴포넌트 테스트는 두지 않는다(프로젝트에 testing-library 가 없고, 상태 하나짜리 토글이다 — GUI 체크리스트로 확인).

- [ ] **Step 4: Commit**

```bash
git add src/pages/settings/Translation.tsx src/locales/ko.json src/locales/en.json src/locales/ja.json
git commit -m "feat(ui): change a saved API key without deleting it first"
```

---

### Task 7: macOS 기본 출력 장치 변경 시 집계 장치 재생성

**Files:**
- Modify: `crates/babelay-engine/csrc/tap.m`

**Interfaces:**
- C ABI 불변: `babelay_tap_start(cb, user, handle_out) -> int`, `babelay_tap_stop(handle)`, `babelay_tap_probe() -> int`. 반환 코드 불변. `stop` 반환 후 콜백이 더 불리지 않는다는 계약 유지(Rust 가 그 뒤 sink 를 해제한다).

- [ ] **Step 1: 핸들 확장과 집계 장치 open/close 분리**

`tap_handle` 에 필드 추가:

```objc
    void *queue;     // dispatch_queue_t — 리스너·재생성·정지가 직렬로 도는 큐 (CFBridgingRetain 소유)
    void *listener;  // AudioObjectPropertyListenerBlock (CFBridgingRetain 소유)
    void *out_uid;   // NSString* — 현재 집계 장치가 물고 있는 기본 출력 UID (CFBridgingRetain 소유)
```

`build.rs` 는 `-fobjc-arc` 로 컴파일한다. `calloc`/`free` 하는 C 구조체에 ObjC 객체 포인터를 직접 두면 ARC 가 소유를 관리하지 못하므로 `void *` + `CFBridgingRetain`/`CFBridgingRelease` 로 수동 소유한다. 읽을 때는 `(__bridge dispatch_queue_t)h->queue`, `(__bridge NSString *)h->out_uid`. 바꿀 때는 먼저 `CFBridgingRelease` 로 놓고 새 값을 `CFBridgingRetain` 한다:

```objc
static void set_out_uid(tap_handle *h, NSString *uid) {
    if (h->out_uid) CFBridgingRelease(h->out_uid);
    h->out_uid = uid ? (void *)CFBridgingRetain(uid) : NULL;
}
```

기존 `babelay_tap_start` 의 "기본 출력 UID → 집계 장치 생성 → 탭 포맷 읽기 → IOProc 생성·시작" 구간을 두 static 함수로 뺀다(본문은 지금 코드 그대로 옮긴다; 실패 시 자기가 만든 것만 되돌리고 `OSStatus`/음수 코드를 돌려준다):

```objc
// 집계 장치·IOProc 만 닫는다. 탭과 콜백은 남긴다(재생성용).
static void close_aggregate(tap_handle *h) {
    if (h->proc) {
        AudioDeviceStop(h->aggregate, h->proc);
        AudioDeviceDestroyIOProcID(h->aggregate, h->proc);
        h->proc = NULL;
    }
    if (h->aggregate) {
        AudioHardwareDestroyAggregateDevice(h->aggregate);
        h->aggregate = 0;
    }
    set_out_uid(h, nil);
}

// 현재 기본 출력 장치로 집계 장치를 만들고 탭 포맷을 읽고 IOProc 을 시작한다.
// 0 이 아니면 실패 — 만든 것은 되돌렸다. (-2 = 포맷 미지원)
static int open_aggregate(tap_handle *h, NSString *tapUUID) {
    NSString *outUID = nil;
    OSStatus st = default_output_uid(&outUID);
    if (st != noErr) return (int)st;
    NSDictionary *aggDesc = @{ /* 기존 내용 그대로, outUID 와 tapUUID 사용 */ };
    st = AudioHardwareCreateAggregateDevice((__bridge CFDictionaryRef)aggDesc, &h->aggregate);
    if (st != noErr) return (int)st;
    /* 기존 탭 포맷 읽기 → h->channels/rate/interleaved 갱신; 실패·미지원이면 close_aggregate(h); return code */
    /* 기존 AudioDeviceCreateIOProcIDWithBlock(...) 블록 그대로 → h->proc */
    /* AudioDeviceStart 실패면 close_aggregate(h); return (int)st */
    set_out_uid(h, outUID);
    return 0;
}
```

`babelay_tap_start` 는 탭 생성 뒤 `h->queue = (void *)CFBridgingRetain(dispatch_queue_create("com.babelay.tap", DISPATCH_QUEUE_SERIAL));` → `open_aggregate(h, desc.UUID.UUIDString)` → 실패면 `babelay_tap_stop(h); return code;` → 성공이면 Step 2 의 리스너 등록 → `*handle_out = h; return 0;`.

- [ ] **Step 2: 기본 출력 리스너**

`babelay_tap_start` 마지막에(IOProc 이 돌기 시작한 뒤):

```objc
    NSString *tapUUID = desc.UUID.UUIDString;
    AudioObjectPropertyListenerBlock listener = ^(UInt32 n, const AudioObjectPropertyAddress *addrs) {
        (void)n, (void)addrs;
        // 이 블록은 h->queue 에서 돈다(AddPropertyListenerBlock 의 큐 인자). stop 은 리스너를 먼저
        // 떼고 같은 큐에서 dispatch_sync 하므로 h 는 여기서 항상 살아 있다.
        NSString *now = nil;
        NSString *cur = (__bridge NSString *)h->out_uid;
        if (default_output_uid(&now) != noErr || !cur || [now isEqualToString:cur]) return;
        NSLog(@"babelay: default output changed %@ -> %@, rebuilding aggregate", cur, now);
        close_aggregate(h);
        int rc = open_aggregate(h, tapUUID);
        // 실패하면 다음 변경 알림에서 다시 시도한다(폴링 없음). 그동안 프레임은 오지 않는다.
        if (rc != 0) NSLog(@"babelay: aggregate rebuild failed (%d)", rc);
    };
    h->listener = (void *)CFBridgingRetain(listener);
    AudioObjectPropertyAddress defAddr = {kAudioHardwarePropertyDefaultOutputDevice,
                                          kAudioObjectPropertyScopeGlobal,
                                          kAudioObjectPropertyElementMain};
    AudioObjectAddPropertyListenerBlock(kAudioObjectSystemObject, &defAddr,
                                        (__bridge dispatch_queue_t)h->queue,
                                        (__bridge AudioObjectPropertyListenerBlock)h->listener);
```

`babelay_tap_stop`:

```objc
void babelay_tap_stop(void *handle) {
    tap_handle *h = (tap_handle *)handle;
    if (!h) return;
    if (h->listener) {
        AudioObjectPropertyAddress defAddr = {kAudioHardwarePropertyDefaultOutputDevice,
                                              kAudioObjectPropertyScopeGlobal,
                                              kAudioObjectPropertyElementMain};
        AudioObjectRemovePropertyListenerBlock(kAudioObjectSystemObject, &defAddr,
                                               (__bridge dispatch_queue_t)h->queue,
                                               (__bridge AudioObjectPropertyListenerBlock)h->listener);
        CFBridgingRelease(h->listener);
        h->listener = NULL;
    }
    // 진행 중인 재생성이 끝난 뒤에 닫는다. 반환 시점에는 IOProc 이 없으므로 콜백도 없다.
    if (h->queue) {
        dispatch_sync((__bridge dispatch_queue_t)h->queue, ^{ close_aggregate(h); });
        CFBridgingRelease(h->queue);
        h->queue = NULL;
    } else {
        close_aggregate(h);  // start 의 초기 실패 경로(큐를 만들기 전)
    }
    if (h->tap) AudioHardwareDestroyProcessTap(h->tap);
    free(h->scratch);
    free(h);
}
```

`babelay_tap_start` 에서 큐는 탭 생성 직후, `open_aggregate` 호출 전에 만든다. 리스너 등록은 `open_aggregate` 성공 뒤 마지막 단계다 — 그래야 start 의 실패 경로가 부르는 `babelay_tap_stop` 이 등록되지 않은 리스너를 떼려 하지 않는다.

- [ ] **Step 3: 컴파일·기존 테스트**

Run: `mise exec -- cargo test -p babelay-engine`
Expected: 빌드 성공(`cc` 가 tap.m 을 다시 컴파일), 기존 테스트 PASS. `captures_some_frames` 는 `--ignored` 로 소리를 내면서 한 번 돌려 프레임이 여전히 온다는 것을 확인한다:

```bash
mise exec -- cargo test -p babelay-engine captures_some_frames -- --ignored --nocapture
```

- [ ] **Step 4: 실기 확인(가능하면)**

`mise exec -- yarn tauri dev` → 라이브 시작 → `say` 로 소리 → 사운드 설정에서 출력 장치를 바꾸거나 헤드폰을 꽂는다 → stderr 에 `default output changed … rebuilding aggregate` 가 찍히고 자막이 이어진다. 불가능하면 GUI 체크리스트(Task 9)로 넘긴다.

- [ ] **Step 5: Commit**

```bash
git add crates/babelay-engine/csrc/tap.m
git commit -m "feat(capture): rebuild the aggregate device when the default output changes (macOS)"
```

---

### Task 8: Windows 기본 장치 폴링 + 재연결

**Files:**
- Modify: `crates/babelay-engine/src/capture/windows.rs`

**Interfaces:**
- `AudioSource for LoopbackSource` 불변. 프레임의 `rate`/`channels` 는 현재 장치 것(장치가 바뀌면 값도 바뀐다 — Task 2 가 받는다).

- [ ] **Step 1: wasapi 0.24 API 이름 확인**

```bash
mise exec -- cargo fetch --target x86_64-pc-windows-msvc
grep -n "pub fn get_id\|pub fn get_default_device\|pub fn stop_stream\|pub fn set_get_eventhandle\|pub struct Handle" ~/.cargo/registry/src/*/wasapi-0.24*/src/api.rs
```

Expected: `Device::get_id(&self) -> WasapiRes<String>`, `DeviceEnumerator::get_default_device(&self, &Direction)`, `AudioClient::stop_stream`, `set_get_eventhandle -> WasapiRes<Handle>`. 이름이 다르면 아래 코드의 호출·타입을 그 이름으로 맞춘다.

- [ ] **Step 2: 구현**

`start` 의 스레드 본문을 다음 구조로 바꾼다(형식 검사·이벤트 모드·`silent` 처리·바이트→f32 변환은 기존 코드 그대로 옮긴다):

```rust
/// 열린 루프백 스트림 하나. 장치가 바뀌면 통째로 다시 만든다.
struct Stream {
    id: String,
    client: wasapi::AudioClient,
    event: wasapi::Handle,
    capture: wasapi::AudioCaptureClient,
    rate: u32,
    channels: u16,
}

fn default_render() -> Result<wasapi::Device, String> {
    DeviceEnumerator::new()
        .map_err(|e| e.to_string())?
        .get_default_device(&Direction::Render)
        .map_err(|e| e.to_string())
}

/// 기본 출력 장치를 루프백 캡처로 연다. 기존 start() 본문의 열기 구간 그대로.
fn open() -> Result<Stream, String> {
    let device = default_render()?;
    let id = device.get_id().map_err(|e| e.to_string())?;
    let mut client = device.get_iaudioclient().map_err(|e| e.to_string())?;
    let mix = client.get_mixformat().map_err(|e| e.to_string())?;
    let rate = mix.get_samplespersec();
    let channels = mix.get_nchannels();
    if mix.get_bitspersample() != 32 || !matches!(mix.get_subformat(), Ok(SampleType::Float)) {
        return Err(format!("unsupported mix format: {} bit, {:?}", mix.get_bitspersample(), mix.get_subformat()));
    }
    client
        .initialize_client(&mix, &Direction::Capture, &StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: 200_000 })
        .map_err(|e| e.to_string())?;
    let event = client.set_get_eventhandle().map_err(|e| e.to_string())?;
    let capture = client.get_audiocaptureclient().map_err(|e| e.to_string())?;
    client.start_stream().map_err(|e| e.to_string())?;
    Ok(Stream { id, client, event, capture, rate, channels })
}
```

스레드 본문:

```rust
        let thread = std::thread::spawn(move || {
            if let Err(e) = wasapi::initialize_mta().ok().map_err(|e| e.to_string()) {
                let _ = ready_tx.send(Err(e));
                return;
            }
            let mut stream = match open() {
                Ok(s) => s,
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };
            let _ = ready_tx.send(Ok(()));
            let mut bytes: VecDeque<u8> = VecDeque::new();
            while !stop2.load(Ordering::Relaxed) {
                // 1초마다 깬다(무음 엔드포인트는 이벤트를 안 준다). 그때 기본 장치가 바뀌었는지 본다.
                let changed = if stream.event.wait_for_event(1000).is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    default_render().and_then(|d| d.get_id().map_err(|e| e.to_string())).is_ok_and(|id| id != stream.id)
                } else {
                    false
                };
                let read = if changed { Err("default device changed".to_string()) } else {
                    stream.capture.read_from_device_to_deque(&mut bytes).map_err(|e| e.to_string())
                };
                let info = match read {
                    Ok(info) => info,
                    Err(e) => {
                        // 장치 제거(AUDCLNT_E_DEVICE_INVALIDATED) 또는 기본 장치 전환: 새 기본 장치로 다시 연다.
                        eprintln!("babelay capture: {e} — reopening the default device");
                        let _ = stream.client.stop_stream();
                        bytes.clear();
                        stream = loop {
                            if stop2.load(Ordering::Relaxed) { return; }
                            match open() {
                                Ok(s) => break s,
                                Err(e) => {
                                    eprintln!("babelay capture: reopen failed: {e}");
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                }
                            }
                        };
                        continue;
                    }
                };
                let n = bytes.len() / 4;
                if n == 0 { continue; }
                let mut samples = Vec::with_capacity(n);
                for _ in 0..n {
                    let mut b = [0u8; 4];
                    for byte in b.iter_mut() { *byte = bytes.pop_front().unwrap_or(0); }
                    samples.push(f32::from_le_bytes(b));
                }
                if info.flags.silent { samples.fill(0.0); }
                sink(Frame { samples, rate: stream.rate, channels: stream.channels });
            }
            let _ = stream.client.stop_stream();
        });
```

`ready_rx.recv()` 처리와 `stop()`·`Drop` 은 그대로. 기존 `// ponytail:` 주석(실패한 핸들의 100 Hz 스핀)은 `wait_for_event` 자리에 그대로 둔다.

- [ ] **Step 3: 격리 크로스 체크**

워크스페이스 전체 `--target x86_64-pc-windows-msvc` 는 `ring` 때문에 불가(2단계 ledger). 격리 크레이트로 확인한다:

```bash
S=/private/tmp/claude-501/-Users-hwpark-Documents-rust-workspace-babelay-app/6e145f5c-5a62-4598-8f3d-315e63cbf4a6/scratchpad/wincheck
mkdir -p $S/src/capture && cd $S
cat > Cargo.toml <<'EOF'
[package]
name = "wincheck"
version = "0.0.0"
edition = "2021"
[dependencies]
thiserror = "2"
[target.'cfg(windows)'.dependencies]
wasapi = "0.24"
EOF
cp /Users/hwpark/Documents/rust-workspace/babelay-app/crates/babelay-engine/src/capture/{mod,windows}.rs src/capture/
printf 'pub mod capture;\n' > src/lib.rs
rustup target list --installed | grep -q x86_64-pc-windows-msvc || rustup target add x86_64-pc-windows-msvc
mise exec -- cargo check --target x86_64-pc-windows-msvc
```

Expected: 오류 없음(경고는 허용). `thiserror` 버전은 `crates/babelay-engine/Cargo.toml` 의 것과 맞춘다. 로컬(macOS) 빌드는 `#[cfg(target_os = "windows")]` 로 이 파일을 컴파일하지 않으므로 `cargo test --workspace` 도 다시 돌려 아무것도 깨지지 않았음을 확인한다.

- [ ] **Step 4: Commit**

```bash
git add crates/babelay-engine/src/capture/windows.rs
git commit -m "feat(capture): follow default device changes and reopen the loopback stream (Windows)"
```

---

### Task 9: 문서 — 상위 스펙 반영, GUI 체크리스트, README

**Files:**
- Modify: `docs/superpowers/specs/2026-09-02-babelay-design.md` §4.1, §4.2, §4.3, §4.4, §11
- Modify: `docs/superpowers/specs/2026-09-04-phase4-passthrough-device-design.md` (상태 줄)
- Create: `docs/superpowers/2026-09-04-phase4-gui-checklist.md`
- Modify: `README.md` (장치 변경 제한 언급이 있으면 제거; `grep -n 장치 README.md` 가 비면 건너뛴다)

- [ ] **Step 1: 상위 스펙**

§4.1 의 문단 "기본 출력 장치가 세션 중에 바뀌면(헤드폰 연결·해제 등) 캡처가 멈출 수 있다 — 2단계 제한이다. 장치 변경 감지와 스트림 재시작은 3단계 백로그로 넘긴다. 그때까지는 정지 후 다시 시작하면 새 장치로 붙는다." 를:

> 기본 출력 장치가 세션 중에 바뀌면(헤드폰 연결·해제, 사운드 설정에서 출력 전환) 캡처 모듈이 스스로 따라간다. macOS 는 `tap.m` 이 `kAudioHardwarePropertyDefaultOutputDevice` 리스너로 집계 장치와 IOProc 만 새 기본 출력으로 재생성한다(탭·콜백 유지, 직렬 큐, 실패 시 다음 알림에서 재시도). Windows 는 읽기 루프가 1초마다 기본 장치 id 를 비교하고, 바뀌었거나 읽기 오류면 새 기본 장치로 다시 연다(실패 시 1초 간격 재시도). 엔진 청커는 프레임의 rate/channels 가 바뀌면 리샘플러를 새로 만든다. UI 알림은 없다(전환 순간의 무음만 남는다).

§4.2 목록에 추가:

> - `Final` 의 언어는 Whisper 감지값 하나가 아니라 최근 Final 3개의 다수결로 확정한다(동률이면 이번 감지값). 번역 건너뛰기(원어 == 타겟)와 히스토리 `lang` 은 이 확정값을 쓴다. 원어를 고정했고 타겟과 같으면 번역 단계 자체를 만들지 않는다(`Started.target_lang = null`).

§4.3 첫 항목의 "(폴백은 stderr 로그)" 를 "(폴백은 stderr 로그 + `CpuFallback{stage:"translate"}` 이벤트를 세션당 한 번 — Live 헤더 `CPU` 배지)" 로.

§4.4 의 문장 "세션 중 기본 출력 장치가 바뀌면 프레임이 끊길 수 있고 엔진은 이를 감지하지 않는다(2단계 제한, §4.1)." 를 "세션 중 기본 출력 장치가 바뀌면 캡처 모듈이 스스로 다시 붙는다(§4.1)." 로.

§11 의 3단계 항목에서 "백로그: 장치 변경 감지(§4.1), 로컬 LLM GPU 폴백의 UI 표시 — … (지금은 stderr 로그만)." 를 "백로그는 4단계에서 처리." 로 줄이고, 4단계 항목을:

> 4. **패스쓰루 안정화 + 장치 변경 자가 복구 + 잔여 백로그**: Final 언어 다수결, 원어 고정 == 타겟 시 번역 단계 생략, macOS/Windows 장치 변경 자가 복구, 리샘플러 재생성, 로컬 LLM CPU 폴백 배지, API 키 변경 버튼, 연결 테스트 비동기화. 스펙 docs/superpowers/specs/2026-09-04-phase4-passthrough-device-design.md — 완료(2026-09-04). Windows 재연결은 크로스 `cargo check` 까지, 실행 검증은 Windows 머신에서.

4단계 스펙의 `상태:` 줄을 `구현 완료(2026-09-04)` 로.

- [ ] **Step 2: GUI 체크리스트**

`docs/superpowers/2026-09-04-phase4-gui-checklist.md`:

```markdown
# 4단계(패스쓰루·장치 변경) GUI 확인 체크리스트 — 2026-09-04

`mise exec -- yarn tauri dev` 로 실행하되, 시스템 오디오 권한은 실행한 터미널에 귀속된다(README 참고).
음성은 `say -r 170 "The quick brown fox jumps over the lazy dog."` / `say -v Yuna "안녕하세요, 오늘 회의를 시작하겠습니다."` 로 낸다.

- [ ] **패스쓰루 즉시 표시** — 원어 `auto`, 타겟 `한국어`, `원문 + 번역`. 한국어 문장을 5개 연속 재생한다. 매 문장이 끝나는 즉시 원문이 뜨고 3초 대기가 없다. 짧은 문장("네.", "좋아요.")을 섞어도 같다.
- [ ] **반대 방향 오감지** — 같은 설정에서 영어 문장 5개. 모두 번역이 붙는다(한 문장만 번역 없이 넘어가는 일이 없다).
- [ ] **원어 고정 == 타겟** — 원어 `영어`, 타겟 `영어`. 라이브 헤더가 `EN → EN` 이고 영어 재생 시 번역 줄·오류 배너 없음. 로컬 모델이 설치돼 있지 않아도 시작된다(번역 단계 없음).
- [ ] **출력 장치 전환** — 라이브 중 `say` 를 반복하면서 시스템 설정 › 사운드 › 출력을 다른 장치로 바꾼다. 자막이 이어지고 터미널에 `default output changed … rebuilding aggregate` 가 한 번 찍힌다.
- [ ] **헤드폰 연결·해제** — 라이브 중 헤드폰(또는 USB/블루투스 출력)을 꽂고 뽑는다. 두 번 모두 자막이 이어진다. 정지·재시작 없이.
- [ ] **정지 경합** — 장치를 바꾼 직후 1초 안에 정지를 누른다. 앱이 멈추지 않고 `Stopped` 가 온다(버튼이 풀린다).
- [ ] **CPU 폴백 배지** — 모델 탭에서 가속을 켠 채로 로컬 번역 모델의 GPU 로드가 실패하는 환경이면(또는 임시로 `LocalLlm::load` 의 GPU 시도를 실패시키는 디버그 빌드) 첫 번역 뒤 Live 헤더에 `CPU` 배지가 뜬다. 정지하면 사라지고, 다음 세션 첫 번역에서 다시 뜬다.
- [ ] **연결 테스트** — 로컬 모델 첫 로드(수 초) 동안 라이브 시작/정지·설정 탭 전환이 버벅이지 않는다. 결과 배너는 3단계와 같다.
- [ ] **API 키 변경** — 클라우드 › 키 저장 후 `변경` → 입력 상자와 `취소` 가 보인다. `취소` 는 배지로 돌아간다. 새 키를 저장하면 배지로 돌아가고 연결 테스트가 새 키로 동작한다. 프로바이더를 바꾸면 편집 상태가 접힌다.
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-09-02-babelay-design.md docs/superpowers/specs/2026-09-04-phase4-passthrough-device-design.md docs/superpowers/2026-09-04-phase4-gui-checklist.md README.md
git commit -m "docs: phase 4 — spec matches the shipped behaviour, GUI checklist"
```

---

## 최종 게이트(모든 태스크 후)

```bash
mise exec -- cargo fmt --all -- --check
mise exec -- cargo clippy --workspace --all-targets -- -D warnings
mise exec -- cargo test --workspace
mise exec -- yarn tsc --noEmit
mise exec -- yarn test
mise exec -- yarn build
```

전부 통과한 뒤 `superpowers:requesting-code-review` 로 `c49d593..HEAD` 범위 리뷰.
