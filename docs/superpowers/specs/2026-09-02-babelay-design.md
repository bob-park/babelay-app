# Babelay 설계 스펙

작성일: 2026-09-02
상태: 승인 대기

## 1. 개요

Babelay는 현재 재생 중인 시스템 오디오를 캡처해 실시간으로 전사하고, 필요하면 번역해서 화면 위 오버레이 자막으로 보여주는 데스크톱 앱이다.

- 대상 OS: macOS 14.2 이상(Apple Silicon), Windows 10 이상(x64)
- 스택: Rust + Tauri 2, React + TypeScript + Tailwind. Python 사이드카는 사용하지 않는다.
- 전사: whisper.cpp(`whisper-rs`). 번역: llama.cpp(`llama-cpp-2`) 로컬 LLM 또는 클라우드 API.
- 디자인 시스템: `docs/design/spotify-design.md`. 확정 목업: `docs/design/mockups/`.

## 2. 확정된 결정 사항

| 항목 | 결정 |
|---|---|
| 번역 방식 | 로컬 LLM(Qwen 3.5 2B/4B, Gemma 3 1B/4B) + 클라우드 API 둘 다 지원 |
| 클라우드 프로바이더 | OpenAI, Anthropic, Gemini, DeepL, Custom(OpenAI 호환) |
| 프론트엔드 | React + TypeScript + Tailwind, UI 라이브러리 없음 |
| 메인 창 | 설정 + 실시간 로그 + 세션 히스토리(저장·검색·내보내기) |
| 원어 처리 | 자동 감지 기본, Korean/English/Japanese로 고정 가능 |
| macOS 캡처 | Core Audio Process Tap(권한: 시스템 오디오 녹음) |
| Windows 캡처 | WASAPI 루프백(권한 없음) |
| 엔진 실행 방식 | Tauri 프로세스 안에 직접 링크(사이드카 없음) |
| 메인 창 레이아웃 | 접이식 좌측 사이드바(목업 01, A) |
| 오버레이 기본 스타일 | 반투명 바, 원문(작게)+번역(크게) 2줄(목업 02, A) |
| 오버레이 표시 모드 | 원문+번역 / 원문 / 번역 |
| 온보딩 모델 목록 | 리스트 행(목업 03, A) |
| 오버레이 위치 | 조정 모드에서 드래그·리사이즈, 비율 좌표 저장(목업 04, B) |
| 앱 아이콘 | 바벨탑 자막 줄(목업 05, C) |
| 메인 창 내비게이션(리디자인) | 떠 있는 둥근 패널 사이드바, 아이콘+라벨, 설정 하위 4개 노출(목업 06-01, B) |
| 모델 행(리디자인) | 행 안 진행 바 + 받은 용량, 상태별 버튼(설치됨은 선택+삭제)(목업 06-02, A) |
| 모니터 선택 | 제거. 조정 모드에서 다른 모니터로 드래그하면 `monitor_id` 자동 저장(목업 06-03) |
| 설정 화면 형식 | 그룹 리스트(라벨 왼쪽, 컨트롤 오른쪽) + 오버레이 미리보기 카드(목업 06-03) |
| 메인 창 구조(2.5) | 고정 사이드바 + 하단 시작·설정·정보, 설정은 탭 4개, daisyUI 5 (2026-09-03 스펙 참조) |

## 3. 전체 구조

프로세스는 하나다. Cargo 워크스페이스에 크레이트 둘을 둔다.

```
babelay-app/
├─ src/                      # React 프론트엔드 (메인·오버레이·온보딩 창 공용)
├─ src-tauri/                # Tauri 앱 크레이트: 창, 트레이, 커맨드, 설정, SQLite
├─ crates/babelay-engine/    # 순수 Rust: 오디오 캡처, 전사, 번역, 모델 관리
├─ assets/icon.svg           # 아이콘 원본
└─ docs/
```

`babelay-engine`은 Tauri에 의존하지 않는다. 입력은 시작/정지 명령과 설정 구조체, 출력은 `tokio::mpsc` 채널로 나오는 `EngineEvent`다. 이 경계 덕에 엔진을 단독으로 테스트할 수 있고, 필요하면 사이드카로 분리할 수 있다.

### 3.1 데이터 흐름

```
시스템 오디오 (mac: Core Audio Tap / win: WASAPI loopback)
  → 리샘플: 48kHz stereo → 16kHz mono f32 (선형 보간 자체 구현, `audio.rs`)
  → 청커: 링버퍼 + 에너지 VAD, 무음 0.6s 또는 최대 8s에서 조각 확정
       확정 전에도 2s마다 미확정 버퍼를 전사해 Partial 발행
  → Whisper → Segment { id, t0_ms, t1_ms, lang, text }
  → 번역기 (건너뛰기 조건: 원어 == 자막 언어, 또는 표시 모드 == 원문)
  → EngineEvent::{Partial, Final, Translated, Status, Error, DownloadProgress}
```

`src-tauri`는 이벤트를 Tauri 이벤트 `engine-event`로 모든 창에 전달하고, `Final`과 `Translated`를 SQLite에 적재한다.

### 3.2 창과 트레이

| 창 | 라우트 | 특성 |
|---|---|---|
| 메인 | `#/main/*` | 접이식 사이드바: 라이브 / 히스토리 / 설정(일반·전사 모델·번역·오버레이) |
| 오버레이 | `#/overlay` | 투명, 테두리 없음, 항상 위, 작업표시줄 제외, 평소 클릭 통과 |
| 온보딩 | `#/onboarding` | 첫 실행 시 메인 창 대신 표시 |

트레이 메뉴: 캡처 시작/정지, 오버레이 켬/끔, 메인 창 열기, 종료. 라벨은 UI 언어를 따른다.

전역 단축키(1차는 고정값): `Cmd/Ctrl+Shift+S` 캡처 시작/정지, `Cmd/Ctrl+Shift+O` 오버레이 켬/끔.

## 4. 엔진 (`babelay-engine`)

### 4.1 오디오 캡처

공통 출력은 `f32` 인터리브 PCM과 샘플레이트다. OS별 모듈 둘.

- **macOS**: Core Audio Process Tap. 탭 생성과 집계 장치 구성은 `cc`로 컴파일하는 ObjC 심(`crates/babelay-engine/csrc/tap.m`)이 맡고, Rust에는 C ABI 세 개(`babelay_tap_start` / `babelay_tap_stop` / `babelay_tap_probe`)로만 노출한다(objc2 바인딩 크레이트는 쓰지 않는다). 전역 탭(모든 프로세스 출력)을 만들고, 비공개 집계 장치에 기본 출력 장치를 서브 장치 겸 메인 서브 장치로, 탭을 탭 목록에 넣어 IOProc으로 읽는다(탭 자동 시작은 쓰지 않는다). Info.plist의 `NSAudioCaptureUsageDescription`으로 첫 탭 생성 시 TCC 프롬프트가 뜬다.
- **Windows**: `wasapi` 크레이트로 기본 출력 장치 루프백.

기본 출력 장치가 세션 중에 바뀌면(헤드폰 연결·해제, 사운드 설정에서 출력 전환) 캡처 모듈이 스스로 따라간다. macOS 는 `tap.m` 이 `kAudioHardwarePropertyDefaultOutputDevice` 리스너로 집계 장치와 IOProc 만 새 기본 출력으로 재생성한다(탭·콜백 유지, 직렬 큐, 실패 시 다음 알림에서 재시도). Windows 는 읽기 루프가 타임아웃으로 깰 때마다(≈1초) 기본 장치 id 를 비교하고, 바뀌었거나 읽기 오류면 새 기본 장치로 다시 연다(실패 시 1초 간격 재시도). 엔진 청커는 프레임의 rate/channels 가 바뀌면 리샘플러를 새로 만든다. UI 알림은 없다(전환 순간의 무음만 남는다). Windows 는 크로스 컴파일(`cargo check --target x86_64-pc-windows-msvc`, 격리 크레이트)까지만 검증했고 실기 검증은 Windows 머신에서 한다.

권한 확인 API `check_audio_permission()`은 실제로 탭 생성을 시도해 결과를 돌려준다. 거부 시 프론트는 `x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture` 딥링크 버튼을 보여준다.

### 4.2 전사

- `whisper-rs`, cargo feature: macOS `metal`, Windows `cuda`. CUDA 빌드는 GPU가 없으면 CPU로 동작하므로 Windows 바이너리는 하나다.
- 설정: 모델 경로, 원어(`auto|ko|en|ja`), GPU 토글. GPU 토글 변경은 컨텍스트를 `use_gpu`로 다시 만든다.
- GPU 초기화 실패 시 CPU로 내려가고 `Status::GpuFallback`을 발행한다.
- `Partial`은 오버레이에 원문만 흐리게 표시한다. 번역은 `Final`에서만 수행한다.
- `Final` 의 언어는 Whisper 감지값 하나가 아니라 최근 Final 3개의 다수결로 확정한다(동률이면 이번 감지값). 번역 건너뛰기(원어 == 타겟)와 히스토리 `lang` 은 이 확정값을 쓴다. 원어를 고정했고 타겟과 같으면 번역 단계 자체를 만들지 않는다(`Started.target_lang = null`).

### 4.3 로컬 번역

- `llama-cpp-2`(`default-features = false, features = ["common"]`; `openmp`는 macOS clang 빌드를 깨뜨려 끈다), 같은 feature 규칙. GPU 토글은 `n_gpu_layers`를 1000 또는 0으로 바꾸고, GPU 로드가 실패하면 0으로 한 번 더 시도한다(폴백은 stderr 로그 + `CpuFallback{stage:"translate"}` 이벤트를 세션마다 한 번(`SharedLlm` 인스턴스당 한 번; 캐시된 모델이 폴백본이어도 다음 세션 첫 번역에서 다시 알린다) — Live 헤더 `CPU` 배지).
- 모델은 첫 번역 시점에 로드하고, 모델이 바뀌기 전까지 세션 종료 후에도 유지한다. 프로세스 전역 캐시(`src-tauri/src/llm.rs`, `LlmCache = Arc<Mutex<Option<{path, gpu, LocalLlm}>>>`, `app.manage`)에 최대 하나를 담고, `translator::build`는 로드 없이 `SharedLlm{cache, path, gpu}`만 만들어 돌려준다 — 그래서 캡처는 GGUF 로드를 기다리지 않고 즉시 시작하고, stop→start 나 연결 테스트는 같은 모델을 다시 읽지 않는다. 경로나 GPU 토글이 다르면 첫 번역에서 갈아 끼운다. 모델 삭제는 파일을 지우기 전에 캐시를 비운다(`llm::evict`; Windows 는 mmap 된 파일을 못 지운다). 시작 전 동기 검사(`precheck`)는 키·파일 존재만 본다.
- 요청마다 새 컨텍스트(`n_ctx` = 입력 + 최대 생성 + 8, 512~4096), 스레드 수 = min(코어, 8). 프롬프트는 모델 채팅 템플릿으로 system("You are a subtitle translator… Output only the translation, one line") + user(직전 원문 2줄 컨텍스트 + 번역 대상)를 렌더하고, 템플릿이 없으면 ChatML. Qwen3 계열(파일명 판별)은 어시스턴트 턴을 빈 `<think></think>` 블록으로 미리 채워 사고를 끈다(Qwen3.5는 `/no_think`를 무시하고 사고 블록 안에서 생성 예산을 다 쓴다 — 2026-09-04 실측). 샘플러 greedy, 최대 토큰 = max(32, 입력 토큰 × 3), EOG에서 중단.
- 출력 후처리(`postprocess`, 클라우드와 공용): `<think>…</think>` 제거, 앞뒤 따옴표·공백 제거, 줄바꿈→공백. 빈 결과는 `Empty` 오류.

### 4.4 스레드와 백프레셔

오디오 콜백 → 청커 스레드 → 전사 스레드 → 번역 스레드(로컬·클라우드 모두 동기 `Translator`, `sync_channel(16)`). 전사 스레드는 `Final`을 **먼저** 낸 뒤 `(id, 원문, 원어)`를 번역 큐에 `try_send` 한다 — 큐가 차면 그 조각은 버린다(원문만 보이고 전사는 계속 흐른다). 번역 큐는 절대 전사를 막지 않는다. 번역 스레드가 `Translated{id, text, lang}`을 낸다. 원어 == 타겟이면 건너뛴다(이벤트 없음). 번역 실패는 `Error{code:"translate"}`를 연속 실패 중 30초에 한 번만 발행하고 원문은 그대로 남는다. 번역 스레드가 있으면 `Stopped`는 번역 스레드가(큐가 닫힌 뒤) 보낸다. 프레임 채널은 unbounded라 과부하에서도 프레임을 버리지 않는다. 전사가 밀리면 `Partial` 실행은 건너뛰고 `Final` 조각은 버리지 않는다. 큐에 오래 머문 조각이 기준을 넘으면 `Lagging`을 한 번 발행하고 회복되면 해제한다 — 경고일 뿐 부하를 덜지는 않는다(load shedding 없음).

정지(`stop_capture`)는 대기 중인 번역을 포기한다(`AtomicBool` 플래그, 번역 루프가 조각마다 확인하고 번역 없이 큐를 비운다) — 세션이 끝난 뒤의 번역은 쓸 데가 없고, 재시도 예산 때문에 정지가 몇 분씩 걸리면 안 된다. 전사 드레인은 그대로 기다린다.

전사 루프는 패닉에 안전하다. `catch_unwind`로 감싸 whisper 쪽이 패닉해도 세션은 `Error{code:"panic"}`을 발행하고 정상적으로 멈춘다.

세션 중 기본 출력 장치가 바뀌면 캡처 모듈이 스스로 다시 붙는다(§4.1). 앱 종료 시 드레인은 3초까지만 기다린다 — 오디오 탭은 그 전에 동기로 풀고, 넘기면 전사 꼬리만 잃는다.

### 4.5 오류

- 치명(모델 로드 실패, 캡처 장치 열기 실패): 세션을 멈추고 `Error{code, message}` 발행.
- 일시(클라우드 429/5xx, 타임아웃 10초): 2회 재시도 후 실패하면 해당 조각은 번역 없이 원문만 표시하고 넘어간다.

## 5. 모델 레지스트리와 다운로드

레지스트리는 엔진(`crates/babelay-engine/src/models.rs`)에 정적 테이블로 내장한다(원격 카탈로그 없음). 필드: `id, kind(asr|llm), name, desc_key, size_bytes, speed(1~5), url, filename, sha256: Option`. `installed(dir, &ModelInfo)`는 파일 존재 + 크기 일치로 판단한다.

### 5.1 전사 모델 (Hugging Face `ggerganov/whisper.cpp`)

| id | 표시명 | 용량 | 속도 |
|---|---|---|---|
| tiny | Whisper Tiny | 75 MB | 5 |
| base | Whisper Base | 142 MB | 4 |
| small | Whisper Small | 466 MB | 3 |
| medium | Whisper Medium | 1.5 GB | 2 |
| large-v3-turbo | Whisper Large v3 Turbo | 1.6 GB | 2 |
| large-v3 | Whisper Large v3 | 3.1 GB | 1 |

### 5.2 번역 모델 (GGUF Q4_K_M, 게이트 없는 미러 사용; 정확한 URL·크기는 구현 시 HEAD 요청으로 확정)

| id | 표시명 | 용량(근사) | 속도 |
|---|---|---|---|
| gemma3-1b | Gemma 3 1B | 0.8 GB | 5 |
| qwen3.5-2b | Qwen 3.5 2B | 1.4 GB | 4 |
| gemma3-4b | Gemma 3 4B | 2.5 GB | 3 |
| qwen3.5-4b | Qwen 3.5 4B | 2.5 GB | 3 |

### 5.3 다운로드

- 저장 위치: 앱 로컬 데이터 디렉터리(`app_local_data_dir`, Windows는 %LOCALAPPDATA%) `models/asr/`, `models/llm/`.
- 엔진 `download.rs`: `reqwest` 스트리밍으로 `.part`에 받고, Range 헤더로 이어받기, `sha256`이 있으면 해시 검증·없으면 크기 검증 후 이름 변경. 취소 시 `.part`를 남겨 다음에 이어받는다.
- `src-tauri` 커맨드: `get_models() -> Vec<ModelStatus { info, installed, in_use, balanced, download: Option<{received, total}> }>`, `download_model(id)`, `cancel_download(id)`, `delete_model(id)`(사용 중이면 거부).
- 이벤트 `model-download { id, received, total, state: downloading|done|error|cancelled, message? }`. 동시 다운로드 1개.

### 5.4 balanced 추천 규칙

`hardware::detect()`가 사양을 읽는다. 시스템 정보는 `sysinfo`(RAM), macOS는 Apple Silicon 여부, Windows는 `nvml-wrapper` 초기화 성공 여부와 VRAM으로 판단한다. 메모리 기준은 Apple Silicon이면 통합 메모리, NVIDIA면 VRAM이다. 실측 총량은 16 GiB를 15.9 GiB로 보고하므로 가장 가까운 GiB로 반올림한다. Windows에서 NVIDIA가 아닌 GPU(AMD·Intel)는 감지하지 않고 CPU 행으로 떨어진다.

| 조건 | 전사 | 번역 |
|---|---|---|
| GPU 있음, 메모리 ≥ 16 GB | large-v3-turbo | qwen3.5-4b |
| GPU 있음, 메모리 ≥ 8 GB | small | qwen3.5-2b |
| 그 외(CPU) | base | gemma3-1b |

이 표는 `ponytail:` 주석으로 표시하고 실측 후 조정한다. 2단계부터 `balanced`는 이 표를 `hardware::detect()` 결과에 적용해 계산한다(레지스트리 고정값 아님).

### 5.5 설정 페이지 표시

설정 > 모델 페이지는 세그먼트(전사/번역)로 목록을 전환한다. 온보딩과 같은 `ModelRow`에 배지 `사용 중`(초록), `설치됨`, `추천`(회색)을 붙이고, 메타 한 줄(용량 · 설명 · 속도 점 5개)을 둔다. 다운로드 중이면 메타에 퍼센트·받은 용량, 아래에 얇은 진행 바. 오른쪽 버튼: 미설치→다운로드, 다운로드 중→취소, 설치됨(미사용)→선택 + 삭제, 사용 중→없음(배지로 표시). 다른 모델을 내려받는 동안 다운로드 버튼은 비활성. 페이지 하단에 GPU 가속 토글. 페이지 상단에는 `hardware::detect()`가 읽은 사양 한 줄(칩 · 메모리 · GPU)을 둔다.

## 6. 번역 프로바이더

```rust
pub trait Translator: Send {
    fn translate(&mut self, req: &TranslateRequest) -> Result<String, TranslateError>;
    fn name(&self) -> &str;
}
pub struct TranslateRequest { text: String, src: String, tgt: String, context: Vec<String> }
pub enum TranslateError { Load(String), Request(String), Http(u16, String), RateLimited, Auth, Timeout, Empty }
```

동기 트레이트다(번역 워커 스레드에서 실행). 구현체(`crates/babelay-engine/src/translate/`): `local::LocalLlm`, `cloud::{OpenAiCompatible, Anthropic, Gemini, DeepL}`. Custom은 `OpenAiCompatible`에 사용자 `base_url`.

| 어댑터 | 엔드포인트 | 인증 | 응답 경로 |
|---|---|---|---|
| OpenAiCompatible | `POST {base_url}/chat/completions` (기본 `https://api.openai.com/v1`, `max_tokens` 512) | `Authorization: Bearer` | `choices[0].message.content` |
| Anthropic | `POST {base_url}/v1/messages` (`max_tokens` 512) | `x-api-key`, `anthropic-version: 2023-06-01` | `content[0].text` |
| Gemini | `POST {base_url}/v1beta/models/{model}:generateContent?key=` (`generationConfig.maxOutputTokens` 512) | 쿼리 `key` | `candidates[0].content.parts[0].text` |
| DeepL | `POST {base_url}/v2/translate` (form: `text`, `target_lang` 대문자·영어는 `EN-US`, `source_lang`, 비어 있지 않으면 `context`) | `Authorization: DeepL-Auth-Key` | `translations[0].text` |

DeepL은 키가 `:fx`로 끝나면 `api-free.deepl.com`, 아니면 `api.deepl.com`. 채팅형 셋은 `system_prompt`/`user_prompt`를 공유하고 응답은 `postprocess`를 거친다. 모델명이 비면 기본값(`gpt-4o-mini` / `claude-haiku-4-5-20251001` / `gemini-2.5-flash`).

- HTTP 타임아웃 10초. `with_retry`: 429(`RateLimited`) / 5xx(`Http(5xx, _)`) / 타임아웃이면 200ms, 600ms 뒤 최대 2회 재시도; 401·403(`Auth`)과 그 외는 즉시 실패. 재시도 판단은 상태 코드를 보고 메시지 문자열을 보지 않는다.
- API 키는 `keyring` 크레이트로 OS 자격 증명 저장소에 저장한다(service `com.babelay.app`, user = 프로바이더 `openai|anthropic|gemini|deepl|custom`). 설정 파일에는 넣지 않는다. 커맨드 `set_api_key`(빈 키는 삭제) / `has_api_key` / `delete_api_key`. 화면은 "저장됨 ●●●●" 상태만 보여준다.
- "연결 테스트"(`test_translation`, 상한 20초)는 설정 그대로 번역기를 조립해 "Good morning."을 한 번 번역하고 `{ok, ms, text, error}`를 돌려준다. 실패 코드: `translation_model_missing` / `api_key_missing` / `base_url_missing` / `display_mode_source` / `translate` / `timeout`.
- 세션 시작의 동기 검사도 같은 코드를 쓴다(`translator::precheck`): 키·모델 존재만 확인한다. 로컬 모델 로드는 첫 번역에서 일어나고 캐시를 세션과 공유한다(§4.3) — 그래서 두 번째 연결 테스트는 로드 없이 끝난다.
- 조각 하나당 요청 하나. 직전에 확정된 원문 2문장을 `context`로 함께 보낸다(채팅형 셋은 유저 프롬프트에, DeepL은 form 필드 `context`에). 로컬 LLM도 같은 형식.
- 세션당 번역은 한 번에 하나만 진행하고 순서를 보존한다.
- 자동 감지 시 원어는 Whisper가 조각마다 돌려주는 언어 코드를 쓴다.

## 7. 프론트엔드

- 하나의 Vite 앱. 창 라벨에 따라 `#/main/*`, `#/overlay`, `#/onboarding`을 렌더한다. 라우팅은 react-router.
- 상태는 zustand 스토어 둘. `settings`는 커맨드 `get_settings/set_settings`와 이벤트 `settings-changed`로 동기화. `session`은 `engine-event`로 라이브 로그와 오버레이 텍스트를 채운다.
- daisyUI 5(CSS 플러그인)만 사용, 아이콘 라이브러리 없음. Tailwind + 네이티브 요소(`<select>`, `<dialog>`, `<input type=range>`). 아이콘은 인라인 SVG 13개(`src/components/icons.tsx`).
- 공용 컴포넌트: `Sidebar`(떠 있는 패널, 접기 상태 저장), `ModelRow`, `SegmentedControl`, `SettingGroup`/`SettingRow`, `ErrorBar`, `DownloadToast`, `PermissionRow`, `ConfirmModal`, `Settings`(탭 셸).
- 디자인 토큰: 카드 반경 12px, 사이드바 패널 14px, 컨트롤은 pill. 필 버튼은 대문자·자간 없이 굵기 600. 보조 텍스트 `#8a8a8a`(다크) / `#6a6a6a`(라이트). 안내 문장·힌트 문구는 두지 않는다(제목과 컨트롤만).

### 7.1 i18n

react-i18next, `src/locales/{ko,en,ja}.json`. "시스템 기본"은 프론트에서 `navigator.language`, 백엔드(트레이)는 `sys-locale`로 같은 규칙으로 해석한다. 지원 밖 언어는 영어.

### 7.2 테마

`system|dark|light`. `html.dark` 클래스 토글 + Tailwind `dark:`. 다크는 디자인 문서 값 그대로, 라이트는 배경 `#ffffff`/`#f5f5f5`, 표면 `#eeeeee`, 텍스트 `#121212`. 초록 액센트는 두 테마 모두 "채우기 + 검정 글자"로만 쓰고, 흰 배경 위 초록 글자는 금지한다.

### 7.3 페이지

- 라이브: 헤더(제목, 시작/정지, 오버레이 켬/끔), "원어 → 자막 언어 · 모델명" 한 줄, 원문/번역 타임라인. 빈 상태 문장 없음.
- 히스토리: 세션 목록(날짜·길이·언어), 상세, 텍스트 검색, TXT/SRT 내보내기, 삭제
- 설정 > 일반: 그룹 리스트(테마, UI 언어) + 단축키 그룹(값만)
- 설정 > 모델(`/settings/models`): 세그먼트 전사/번역, 모델 행 목록(다운로드·삭제·선택), GPU 가속 토글(macOS "Apple Silicon 가속", Windows "NVIDIA GPU 가속")
- 설정 > 번역: 세그먼트 로컬/클라우드. 로컬은 현재 모델 한 줄(변경은 모델 페이지), 클라우드는 그룹 리스트(프로바이더·모델·Base URL·키·연결 테스트)
- 설정 > 오버레이: 미리보기 카드 + 위치 조정 버튼, 그룹 1(표시 모드, 자막 언어, 원어), 그룹 2(글자 크기, 배경 투명도). 모니터 선택 없음

### 7.4 오버레이 창

조정 모드에서만 클릭 통과를 끄고 드래그 영역과 모서리 리사이즈 핸들을 보여준다. 위치는 `{monitor_id, x_ratio, y_ratio, w_ratio}`로 저장해 해상도가 바뀌어도 비율로 복원한다. 다른 모니터로 드래그하면 위치 확정 시 백엔드가 현재 모니터를 읽어 `monitor_id`를 갱신하므로 별도의 모니터 선택 UI는 없다. 자막 상자는 항상 두 줄까지다(`overlayLines`). 번역이 붙어 있으면 `both`는 번역(큰 줄) + 원문(작은 줄)이고, 번역이 없으면 원문(큰 줄) + `Partial`(작은 줄, 흐리게)이다 — 번역된 한 쌍이 두 줄을 다 쓰므로 그때 `Partial` 줄은 나오지 않는다. `source`는 언제나 원문 + `Partial`, `target`은 번역 한 줄이고 번역이 없으면(패스쓰루·안전 상한 만료) 원문으로 내려간다. 무음 6초 후 페이드아웃한다. 원문과 번역은 한 세트로 동시에 나타난다(`pairForOverlay`): 오버레이는 번역이 붙은 세트만 그린다 — 새 `Final`이 와도 번역이 없으면 화면은 바뀌지 않고, 번역이 도착하는 순간 그 원문+번역으로 함께 교체된다(가장 최근에 번역이 붙은 `Final`). 번역이 15초가 지나도 오지 않으면(실패·큐에서 버려짐) 그 원문만 그린다 — 정상 흐름에서는 걸리지 않는 안전 상한이다(2026-09-04, 4단계 실기 피드백으로 3초 유지 후 원문 폴백 규칙을 교체). 번역 타겟은 `Started{target_lang}`이 알려준다(프론트가 UI 언어로 다시 유추하지 않는다). 번역이 올 수 없는 경우(`target_lang` 없음, 원어 == 타겟)는 기다리지 않는다. 대기 중에만 100ms 타이머로 다시 그린다.

### 7.5 온보딩

언어 → 권한(macOS만, Windows는 건너뜀) → 전사 모델 → 번역 모델(건너뛰기 가능) → 완료. 상단은 단계 수만큼의 진행 바, 각 단계는 제목과 컨트롤만(설명 문장 없음). 모델 단계는 `ModelRow`로 선택하고, 미설치 모델이면 버튼이 "N MB 받고 계속"이 되어 다운로드를 시작하고 행에 진행률을 보여준 뒤 완료되면 다음 단계로 넘어간다. 설치된 모델이면 "계속". 완료 시 `onboarding_done=true`를 저장하고 메인 창으로 전환한다.

### 7.6 설정 스키마

앱 설정 디렉터리의 `settings.json`. `version` 필드로 마이그레이션에 대비한다.

```
version: 1
general:     theme(system|dark|light), ui_language(system|ko|en|ja), onboarding_done
asr:         model_id, gpu(bool), source_lang(auto|ko|en|ja)
translation: backend(local|cloud), local_model,
             cloud: { provider(openai|anthropic|gemini|deepl|custom), model, base_url }
overlay:     enabled, monitor_id, x_ratio, y_ratio, w_ratio,
             display_mode(both|source|target), subtitle_lang(system|ko|en|ja),
             font_size, bg_opacity
```

## 8. 저장소

`rusqlite`(bundled, fts5). 파일은 로컬 데이터 디렉터리의 `history.sqlite`.

```
sessions(id, started_at, ended_at, src_lang, tgt_lang, asr_model, translator)
segments(id, session_id, t0_ms, t1_ms, lang, src_text, tgt_text)
segments_fts(src_text, tgt_text)   -- 히스토리 검색, external content(content='segments')
```

`segments_fts`는 `segments`를 원본으로 하는 external-content FTS5 테이블이고 INSERT·DELETE·UPDATE 트리거(`segments_ai/ad/au`)로 동기화한다. `Final`이 행을 만들고(`insert_segment`가 행 id를 돌려줌, 세션 상태의 `final_rows`에 엔진 id → 행 id 기록), 같은 id의 `Translated`가 `tgt_text`를 UPDATE 한다. `sessions.translator`는 `local:<model_id>` / `cloud:<provider>/<model>` / NULL(번역 없음). 내보내기: SRT 블록은 원문 줄 + (있으면) 번역 줄, TXT는 `원문\t번역`.

히스토리 DB는 시작 시 선택 사항이다. 열기에 실패해도 앱은 뜨고 캡처·오버레이는 그대로 동작하며 히스토리 기능만 빠진다.

TXT/SRT 내보내기는 `segments`를 순서대로 포매팅하는 함수 하나.

## 9. 빌드, 서명, 아이콘

### 9.1 macOS

`tauri build`가 서명과 공증을 처리한다. 인증서는 환경변수로 받는다.

```
APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
APPLE_ID / APPLE_PASSWORD(앱 암호) / APPLE_TEAM_ID
```

Hardened Runtime 엔타이틀먼트 `com.apple.security.device.audio-input`, Info.plist `NSAudioCaptureUsageDescription`, `LSMinimumSystemVersion=14.2`. 산출물은 Apple Silicon `.dmg`.

### 9.2 Windows

NSIS 인스톨러, 서명 없음. cudart/cublas/cublasLt DLL을 `bundle.resources`로 동봉한다. 빌드는 로컬에서만 한다(CI 없음). Windows 빌드는 CUDA 툴킷이 설치된 Windows 머신에서 수행한다.

### 9.3 아이콘

`assets/icon.svg`(목업 05의 C 시안: 근흑 그라데이션 배경 위 자막 줄 4개, 최상단 줄만 초록)에서 `tauri icon`으로 `.icns/.ico/png` 세트를 생성한다. 트레이용은 단색 템플릿 PNG를 별도로 둔다.

## 10. 테스트

- 엔진 단위 테스트: 청커/VAD(합성 사인파+무음), 리샘플러, 레지스트리 무결성(id 유일, https), balanced 규칙 표, SRT 포매터, 클라우드 어댑터 요청 본문 조립
- 모델 파일이 필요한 whisper/llama 통합 테스트는 `#[ignore]`
- 프론트: vitest로 스토어, 세 로케일 파일의 키 집합 일치 검사
- 수동 체크리스트: macOS 권한 흐름, GPU 폴백, 다중 모니터, 조정 모드

## 11. 구현 단계

각 단계는 별도 구현 계획으로 작성한다.

1. **앱 셸**: Tauri 2 + React 스캐폴드, 테마, i18n, 설정 파일, 접이식 사이드바, 트레이, 온보딩 골격, 오버레이 창(조정 모드 포함), 아이콘, 서명 설정 — 완료(2026-09-03)
1.5. **모델 다운로드 + UI 리디자인**: 엔진 모델 레지스트리·다운로드(진행률·이어받기·취소·삭제), 설치/사용 중 배지, 모니터 선택 제거, 떠 있는 패널 사이드바·그룹 리스트 설정·모델 페이지·온보딩 리디자인, 문구 정리
2. **전사 엔진**: 오디오 캡처(mac/win), 청커, whisper, 사양 기반 balanced, GPU 토글, 라이브 페이지, SQLite·히스토리 — 완료(2026-09-03), 런타임 캡처 검증은 coreaudiod 재시작 후 보류
   - Windows 캡처는 캡처 모듈만 크로스 타깃 `cargo check`로 확인했다(워크스페이스 전체 check는 `ring`이 막는다). 런타임 검증은 Windows 머신에서.
2.5. **UI 리디자인 + 백그라운드 온보딩 + 오버레이 수정**: 스펙 docs/superpowers/specs/2026-09-03-phase2.5-ui-onboarding-design.md — 완료(2026-09-03)
3. **번역**: 로컬 llama, 클라우드 어댑터 4종(Custom은 OpenAI 호환 공용), keyring, 설정 > 번역(키 저장·연결 테스트), 오버레이 한 세트 규칙, 히스토리 번역 저장·검색·내보내기 — 완료(2026-09-03). 백로그는 4단계에서 처리.
4. **패스쓰루 안정화 + 장치 변경 자가 복구 + 잔여 백로그**: Final 언어 다수결, 원어 고정 == 타겟 시 번역 단계 생략, macOS/Windows 장치 변경 자가 복구, 리샘플러 재생성, 로컬 LLM CPU 폴백 배지, API 키 변경 버튼, 연결 테스트 비동기화. 스펙 docs/superpowers/specs/2026-09-04-phase4-passthrough-device-design.md — 완료(2026-09-04). Windows 재연결은 크로스 `cargo check` 까지, 실행 검증은 Windows 머신에서. 백로그: 장치 끊김 뒤 타임스탬프가 끊긴 시간만큼 앞당겨진다(`Chunker::consumed` 가 샘플 기준); 집계 장치 재생성이 계속 실패하면 다음 장치 변경까지 무음이고 UI 알림이 없다; Windows 재연결 실패 로그가 1 Hz 로 무한 반복; 원어 고정 == 타겟이어도 `precheck` 가 번역 모델·키를 요구한다.

## 12. 범위 밖 (1차)

원격 모델 카탈로그, 자막 스타일 프리셋 추가(외곽선·캡슐), 단축키 사용자 지정, 자동 업데이트, 마이크 입력, 로그인 시 자동 실행.
