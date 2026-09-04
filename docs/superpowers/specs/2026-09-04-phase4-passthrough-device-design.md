# Babelay 4단계: 번역 패스쓰루 안정화 + 장치 변경 자가 복구 + 잔여 백로그 설계

작성일: 2026-09-04
상태: 설계 승인(2026-09-04), 구현 전
상위 스펙: `docs/superpowers/specs/2026-09-02-babelay-design.md` (충돌 시 이 문서가 우선; 구현 완료 후 상위 스펙 §4.1·§4.2·§4.3·§4.4·§11 에 반영한다)
브랜치: main (이전 단계와 같은 사용자 지시)

## 1. 목표

1. **패스쓰루**: 원어가 타겟과 같으면 번역기를 부르지 않고 원문을 바로 보여준다. 코드상 규칙은 3단계에 이미 있으나(`translate_loop` 의 `lang != tgt`, 오버레이 `pending()`), Whisper 가 짧은 Final 에서 언어를 오감지하면 번역이 호출되고 오버레이가 최대 3초 기다린다. 판정을 안정화한다.
2. **장치 변경 자가 복구**: 세션 중 기본 출력 장치가 바뀌어도(헤드폰 연결·해제, 사운드 설정에서 출력 전환) 캡처가 끊기지 않는다. 2단계부터 이어진 백로그(상위 스펙 §4.1).
3. **로컬 LLM CPU 폴백 표시**: GPU 로드가 실패해 CPU 로 내려갔음을 Live 헤더 배지로 알린다(지금은 stderr 로그만).
4. **API 키 변경 버튼**(3단계 리뷰 M11).
5. **연결 테스트 비동기화**(3단계 리뷰 M7).

## 2. 확정 결정

| 항목 | 결정 |
|---|---|
| 언어 판정 | 엔진이 최근 Final 3개의 감지 언어 다수결로 이번 Final 의 `lang` 을 확정한다(1-A). Whisper 힌트 고정(1-B)은 `auto` 의 언어 전환 의미를 잃어 제외. 오버레이가 번역을 안 기다리는 방식(1-C)은 §7.4 "한 세트 동시 교체" 를 깨므로 제외 |
| 원어 고정 == 타겟 | 번역 단계를 만들지 않는다(`Started.target_lang = null`) |
| 장치 변경 | 캡처 모듈이 스스로 복구한다(2-A). `AudioSource` 트레이트·엔진 이벤트는 바뀌지 않는다 |
| 포맷 변경 | 청커가 프레임의 `rate`/`channels` 가 바뀌면 리샘플러를 새로 만든다 |
| 폴백 알림 | 새 이벤트 `CpuFallback { stage }`. 프론트는 기존 `gpuFallback` 배지("CPU")를 재사용한다 |
| 키 변경 | `저장됨` 배지 옆 `변경` 버튼 → 입력 상자 복귀. 저장은 덮어쓰기(백엔드 변경 없음) |
| 연결 테스트 | `async fn` + `tauri::async_runtime::spawn_blocking`. 워커 스레드·20초 상한은 그대로 |

## 3. 언어 판정 (엔진, `engine.rs`)

### 3.1 다수결

`transcribe_loop` 가 `VecDeque<String>`(용량 3)에 Final 마다 Whisper 감지 언어를 넣고, 이번 Final 의 확정 언어를 다음 규칙으로 정한다.

- 창(최근 3개, 이번 포함)에서 가장 많은 언어. 동률이면 이번 감지값.
- 창이 하나뿐이면(첫 Final) 이번 감지값.
- 원어가 고정돼 있으면(`lang: Some(_)`) Whisper 가 그 언어를 돌려주므로 다수결도 같은 값이다. 별도 분기 없음.

확정 언어가 `Final.lang` 으로 나가고 번역 큐 `(id, text, lang)` 에도 같은 값이 실린다. 오버레이 `pending()`·히스토리 `lang` 컬럼은 이미 `Final.lang` 을 보므로 프론트 수정 없음. `Partial.lang` 은 그대로 감지값(표시에 안 쓴다).

예: 영어 발화, 타겟 `ko`. 감지 `[en, en, cy]` → 세 번째 Final 은 `en` 으로 확정 → 번역된다(지금은 `cy→ko` 번역 요청). 한국어 발화, 타겟 `ko`. 감지 `[ko, ko, ja]` → `ko` → 패스쓰루(지금은 3초 대기 후 원문).

한계(ponytail): 언어가 세션 중 바뀌면 Final 2개만큼 늦게 따라간다. 오감지된 조각의 전사 텍스트 자체는 고치지 못한다.

### 3.2 원어 고정 == 타겟

`session.rs` `start`: `tgt_lang` 은 번역이 켜져 있고 **원어가 `auto` 이거나 타겟과 다를 때만** `Some`. 같으면 `None` → 엔진이 번역 스레드를 만들지 않고 `Started.target_lang = null` → 오버레이는 기다리지 않는다. `translator::precheck` 는 그대로 돈다(설정이 잘못되면 시작 전에 알려주는 편이 낫다). 히스토리 `tgt_lang` 라벨은 기존 폴백(`subtitle_lang`)을 쓴다.

## 4. 장치 변경 자가 복구

### 4.1 macOS (`csrc/tap.m`)

- `babelay_tap_start` 가 `kAudioObjectSystemObject` 에 `kAudioHardwarePropertyDefaultOutputDevice` 리스너(`AudioObjectAddPropertyListenerBlock`)를 건다. 리스너는 핸들의 직렬 `dispatch_queue_t` 에서 돈다.
- 리스너 본문 `rebuild(h)`: 현재 IOProc 정지·파기, 집계 장치 파기 → 새 기본 출력 UID 로 집계 장치 재생성 → 탭 포맷 다시 읽어 `channels`/`rate`/`interleaved` 갱신 → IOProc 재생성·시작. **탭과 콜백(`cb`, `user`)은 유지**한다. 재생성이 실패하면 stderr 로그를 남기고 다음 변경 알림에서 다시 시도한다(폴링 없음).
- 새 기본 출력 UID 가 현재 것과 같으면(같은 장치에 대한 중복 알림) 아무것도 하지 않는다.
- `babelay_tap_stop` 은 리스너를 먼저 제거한 뒤 같은 직렬 큐에서 `dispatch_sync` 로 나머지를 정리한다. 그래야 정지와 재생성이 겹치지 않고, 반환 시점에 콜백이 더 이상 불리지 않는다는 Rust 쪽 계약(`stop()` 뒤 sink 해제)이 유지된다.
- 집계 장치 재생성에는 `kAudioAggregateDeviceUIDKey` 가 필요하다. 탭 UUID 기반 값을 그대로 쓰되 파기 후 생성이므로 충돌하지 않는다.

### 4.2 Windows (`capture/windows.rs`)

- 읽기 루프는 이미 `wait_for_event(1000)` 으로 1초마다 깬다. 타임아웃으로 깰 때마다 `DeviceEnumerator::get_default_device(Render).get_id()` 를 현재 장치 id 와 비교한다.
- id 가 다르거나 `read_from_device_to_deque` 가 오류를 돌려주면(장치 제거 시 `AUDCLNT_E_DEVICE_INVALIDATED`) 현재 클라이언트를 `stop_stream` 하고 새 기본 장치로 `initialize_client`~`start_stream` 을 다시 한다. 실패하면 1초 뒤 재시도, 재시도 중에도 `stop` 플래그를 확인한다.
- 재시작 코드는 시작 경로와 같은 함수(`open(device) -> Result<(client, event, capture, rate, channels)>`)를 쓴다. 첫 `open` 실패는 지금처럼 `start()` 오류로, 이후 실패는 로그와 재시도로 처리한다.
- 실행 검증은 Windows 머신에서(3단계와 같은 제한). 이번 단계는 크로스 타깃 `cargo check` 까지.

### 4.3 엔진 (`engine.rs` `chunker_loop`)

리샘플러를 첫 프레임에 고정하지 않는다. 프레임의 `rate` 또는 `channels` 가 직전과 다르면 `Resampler::new` 로 교체한다(직전 보간 샘플 하나를 잃는다 — 장치 전환 순간의 수 ms, 무시). 청커 상태는 유지한다(진행 중 발화가 끊기지 않는다).

### 4.4 UI

이벤트 없음. 전환 순간 수백 ms 의 프레임이 비는 것은 무음으로 흡수된다. 재생성 실패는 stderr 만.

## 5. 로컬 LLM CPU 폴백 표시

- `EngineEvent::CpuFallback { stage: String }` 추가(serde 태그 `cpu_fallback`). 이번 단계에서 `stage` 는 `"translate"` 만 쓴다. Whisper 폴백은 기존 `Started.gpu_fallback` 그대로.
- `llm.rs`: `Loaded` 에 `fell_back: bool` 을 기록한다. `LlmCache` 가 `Option<AppHandle>` 을 든다(`LlmCache::new(app)`; 테스트의 `Default` 는 `None` = 이벤트 안 냄). `SharedLlm` 에 `notified: bool` 을 더하고 `SharedLlm::new(cache, path, gpu)` 로 만든다. `translate` 에서 (a) 방금 로드했고 폴백했거나 (b) 캐시에 이미 있는 모델이 폴백본이면, 이 `SharedLlm` 인스턴스에서 한 번만 `app.emit("engine-event", CpuFallback{stage:"translate"})` 를 낸다 — 세션마다 `SharedLlm` 이 새로 만들어지므로 세션마다 한 번이다. 연결 테스트의 `SharedLlm` 도 같은 이벤트를 내지만 프론트가 캡처 중이 아니면 무시한다.
- `translator::build` 시그니처는 그대로(`lib.rs` 가 `LlmCache::new(app.handle().clone())` 으로 등록한다).
- 프론트 `types.ts` 유니언에 `{ type: "cpu_fallback"; stage: string }`. `session.ts` 리듀서: `capturing` 이면 `gpuFallback: true`, 아니면 무시. Live 헤더의 `CPU` 배지가 그대로 켜진다. 문구 변경 없음.

## 6. API 키 변경 버튼

`Translation.tsx`: 로컬 state `editing: boolean`. `saved && !editing` 이면 `저장됨` 배지 + `변경` + `삭제`. `변경` 을 누르면 `editing = true` → 입력 상자 + `저장`(기존 `saveKey`; 성공 시 `editing = false`) + `취소`. 프로바이더가 바뀌면 `editing = false`. 로케일 3개에 `translation.changeKey` 추가, 취소는 기존 `common.cancel`.

## 7. 연결 테스트 비동기화

`commands.rs` `test_translation`: `#[tauri::command] pub async fn` 으로 바꾸고 `tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(TEST_TIMEOUT)).await` 로 기다린다. 워커 스레드·채널·20초 상한·`"timeout"` 오류 코드는 그대로. 새 의존성 없음(`async_runtime` 은 tauri 재수출).

## 8. 테스트

- 엔진: 다수결은 `LangVote` 로 분리해 단위 테스트한다 — `[en, en, cy] → en`, 첫 Final 은 감지값 그대로, `[en, ko]` 동률은 이번 값, `[en, en, ko] → en` 뒤 `[en, ko, ko] → ko`. `transcribe_loop` 는 `Final.lang` 과 번역 큐에 같은 확정값을 쓴다(기존 파이프라인 테스트는 Final 하나만 내므로 그대로).
- 엔진: `chunker_loop` — 48k/2ch 프레임 뒤 44.1k/1ch 프레임을 넣어도 패닉 없이 16k 모노가 이어진다(출력 샘플 수가 두 구간 합에 근사).
- src-tauri: `session::start` 의 `tgt_lang` 계산을 순수 함수로 빼서 `(source "en", tgt "en") → None`, `("auto", "en") → Some("en")`, `("ko", "en") → Some("en")` 확인.
- 프론트(vitest): 리듀서 `cpu_fallback` — 캡처 중이면 `gpuFallback` 켜짐, idle 이면 변화 없음. 키 `변경` 버튼은 상태 하나짜리 토글이고 프로젝트에 컴포넌트 테스트 도구(testing-library)가 없으므로 tsc + GUI 체크리스트로 확인한다. 로케일 키 집합 일치 테스트는 기존 것이 잡는다.
- tap.m 재생성과 Windows 재연결은 자동 테스트 없음 → GUI 체크리스트(헤드폰 연결/해제, 사운드 설정에서 출력 장치 전환, 전환 후 자막이 이어지는지).
- 게이트: `cargo test --workspace`, `cargo clippy`, `cargo fmt --check`, `yarn test`, `tsc`, `yarn build`, Windows 캡처 모듈 크로스 `cargo check`.

## 9. 문서 반영(구현 후)

- 상위 스펙 §4.1: "2단계 제한 … 3단계 백로그" 문단을 자가 복구 설명으로 교체. §4.2: Final 언어 다수결. §4.3: 폴백 이벤트. §4.4: "장치 변경 감지하지 않는다" 문장 삭제, 리샘플러 재생성. §11: 4단계 항목.
- `docs/superpowers/2026-09-04-phase4-gui-checklist.md`, `2026-09-04-phase4-sdd-ledger.md`.
- README 의 알려진 제한에서 장치 변경 항목 제거.

## 10. 범위 밖

Whisper 언어 힌트 고정, 세션 중 언어 전환 즉시 반영, 장치 변경 UI 알림, 마이크 입력, 3단계 ledger 의 나머지 minor(M6 `n_ctx` 주석, `SharedLlm` 뮤텍스 범위, `LlmCache` 자동 테스트).
