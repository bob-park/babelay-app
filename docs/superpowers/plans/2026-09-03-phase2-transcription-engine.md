# Babelay 2단계: 전사 엔진 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 시스템 오디오를 캡처해 Whisper로 전사하고, 라이브 페이지·오버레이에 실시간 자막(원문)을 띄우며, 세션을 SQLite에 저장·검색·내보내기하고, 시스템 사양 기반으로 추천 모델을 고른다. 번역은 3단계.

**Architecture:** `babelay-engine`에 `audio`(리샘플·청커/VAD), `capture`(macOS Core Audio Process Tap via ObjC 심, Windows WASAPI 루프백), `transcribe`(whisper-rs), `engine`(스레드 오케스트레이션 + `EngineEvent`), `hardware`(사양 감지·balanced)를 넣는다. `src-tauri/session.rs`가 엔진을 시작/정지하고 `engine-event`로 모든 창에 중계하며, `history.rs`가 `Final`을 SQLite에 적재한다. 프론트는 `useSession` 스토어가 `engine-event`를 받아 라이브 타임라인·오버레이·상태 점을 채운다.

**Tech Stack:** whisper-rs 0.16 (features `metal`/`cuda`), objc2 없이 `cc`로 컴파일하는 ObjC 심(macOS), wasapi 0.24(Windows), sysinfo 0.39, nvml-wrapper 0.13(Windows), rusqlite 0.40(bundled = FTS5 포함), Tauri 2.11, React 19, zustand 5. cmake는 `.mise.toml`로 프로젝트에 고정.

**Spec:** `docs/superpowers/specs/2026-09-02-babelay-design.md` §3.1, §3.2, §4, §5.4, §5.5(사양 한 줄), §7.3(라이브·히스토리), §7.4, §8, §11 item 2.

## Global Constraints

- 대상 OS macOS 14.2+(Apple Silicon), Windows 10+(x64). 이 머신은 macOS라 Windows 코드는 `cargo check --target x86_64-pc-windows-msvc`로만 검증한다.
- 셸에 mise가 활성화되어 있지 않으면 yarn·cargo 모두 `mise exec -- …`로 실행한다(cmake가 mise 경유로 PATH에 들어가야 whisper.cpp가 빌드된다).
- 검증 게이트(모든 태스크 공통): `mise exec -- cargo test --workspace`, `mise exec -- cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `mise exec -- yarn tsc --noEmit`, `mise exec -- yarn test`, `mise exec -- yarn build`. Task 3 이후 `mise exec -- cargo check -p babelay-engine --target x86_64-pc-windows-msvc`도 포함.
- `babelay-engine`은 Tauri에 의존하지 않는다. 엔진의 유일한 출력은 `EngineEvent` 채널.
- 오디오 파이프라인 상수: 16kHz 모노 f32, VAD 프레임 20ms, 무음 확정 0.6s, 최대 조각 8s, Partial 주기 2s, Lagging 기준 10s, 오버레이 페이드 6s.
- 모델 파일이 필요한 테스트는 `#[ignore]`이고 `BABELAY_TEST_MODEL` 환경변수로 경로를 받는다.
- 디자인 규칙 유지: UI 라이브러리 없음, 초록은 채우기+검정 글자, 안내 문장 없음, 세 로케일 키 집합 동일.
- 커밋 접두어 `feat:`/`fix:`/`test:`/`docs:`, 메시지 끝에 빈 줄 후
  `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` 와
  `Claude-Session: https://claude.ai/code/session_01SgeKYTWA8jSmJgyUboWFAj`.

---

## 파일 구조

```
.mise.toml                         cmake 추가
crates/babelay-engine/
├─ Cargo.toml                      whisper-rs, sysinfo, serde; [target.macos] cc(build), [target.windows] wasapi, nvml-wrapper
├─ build.rs                        macOS: csrc/tap.m 컴파일
├─ csrc/tap.m                      Core Audio Process Tap ObjC 심 (C ABI 3개)
└─ src/
   ├─ lib.rs                       pub mod audio, capture, transcribe, engine, hardware (+ models, download)
   ├─ audio.rs                     Resampler, Chunker, ChunkEvent
   ├─ capture/mod.rs               AudioSource trait, Frame, default_source(), probe_permission()
   ├─ capture/macos.rs             TapSource (extern "C" 바인딩)
   ├─ capture/windows.rs           LoopbackSource (wasapi)
   ├─ transcribe.rs                Transcriber trait, Segment, WhisperTranscriber
   ├─ engine.rs                    Engine, EngineConfig, EngineEvent, EngineHandle
   └─ hardware.rs                  HwInfo, detect(), balanced()
src-tauri/
├─ Cargo.toml                      babelay-engine features per OS, rusqlite
├─ tauri.windows.conf.json         CUDA DLL resources
└─ src/
   ├─ session.rs                   SessionState, start/stop/toggle, engine-event 중계, Final → history
   ├─ history.rs                   SQLite: sessions/segments/segments_fts, 조회·검색·내보내기·삭제
   ├─ commands.rs                  start_capture, stop_capture, capture_state, get_hw_info, history_* , check_audio_permission(실제)
   ├─ tray.rs                      toggle_capture → session::toggle
   └─ models.rs                    balanced → hardware::balanced
src/
├─ lib/types.ts                    EngineEvent, Segment, SessionSummary, HwInfo
├─ lib/tauri.ts                    새 커맨드 래퍼
├─ lib/session.ts                  engine-event 리듀서(순수) + 스토어
├─ pages/main/Live.tsx             타임라인 + 상태 필
├─ pages/main/History.tsx          목록/상세/검색/내보내기/삭제
├─ pages/OverlayWindow.tsx         실제 텍스트 + 페이드
├─ pages/settings/Models.tsx       사양 한 줄
├─ pages/Onboarding.tsx            권한 단계 실제 결과
├─ locales/*.json                  새 키
└─ test/session.test.ts            리듀서 테스트
```

---

### Task 1: 리샘플러와 청커(VAD)

**Files:**
- Create: `crates/babelay-engine/src/audio.rs`
- Modify: `crates/babelay-engine/src/lib.rs` (`pub mod audio;`)

**Interfaces:**
- Produces:
  ```rust
  pub const TARGET_RATE: u32 = 16_000;
  pub struct Resampler { /* src_rate, channels, phase state */ }
  impl Resampler { pub fn new(src_rate: u32, channels: u16) -> Self; pub fn push(&mut self, interleaved: &[f32], out: &mut Vec<f32>); }
  pub enum ChunkEvent { Partial { pcm: Vec<f32>, start_ms: u64 }, Final { pcm: Vec<f32>, start_ms: u64, end_ms: u64 } }
  pub struct Chunker { /* … */ }
  impl Chunker { pub fn new() -> Self; pub fn push(&mut self, mono16k: &[f32]) -> Vec<ChunkEvent>; pub fn flush(&mut self) -> Option<ChunkEvent>; }
  ```
  규칙: 다운믹스는 채널 평균, 리샘플은 선형 보간. 청커는 20ms 프레임 RMS > 0.01 이면 음성. 음성이 한 번이라도 있었고 무음이 0.6s 이어지면 `Final`; 버퍼가 8s에 이르면 `Final`; 음성이 있고 마지막 Partial 이후 2s 지났으면 `Partial`(버퍼 전체 복사). 음성 없이 1s 이상 무음만 쌓이면 버린다. `flush`는 남은 음성 버퍼를 `Final`로 낸다.

- [ ] **Step 1: 실패하는 테스트**

`audio.rs` 하단:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, secs: f32, amp: f32) -> Vec<f32> {
        (0..(rate as f32 * secs) as usize).map(|i| amp * (i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin()).collect()
    }
    fn silence(secs: f32) -> Vec<f32> { vec![0.0; (TARGET_RATE as f32 * secs) as usize] }

    #[test]
    fn resampler_downmixes_and_halves_48k_to_16k() {
        let mut r = Resampler::new(48_000, 2);
        let stereo: Vec<f32> = (0..48_000 * 2).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
        let mut out = Vec::new();
        r.push(&stereo, &mut out);
        assert!((out.len() as i64 - 16_000).abs() <= 2, "got {}", out.len());
        assert!(out.iter().all(|s| s.abs() < 1e-6), "stereo average of ±0.5 must be 0");
    }

    #[test]
    fn resampler_44100_produces_about_16000_per_second() {
        let mut r = Resampler::new(44_100, 1);
        let mut out = Vec::new();
        r.push(&sine(44_100, 1.0, 0.5), &mut out);
        assert!((out.len() as i64 - 16_000).abs() <= 2, "got {}", out.len());
    }

    #[test]
    fn chunker_finalizes_after_silence() {
        let mut c = Chunker::new();
        let mut ev = c.push(&sine(TARGET_RATE, 1.0, 0.3));
        ev.extend(c.push(&silence(0.7)));
        let finals: Vec<_> = ev.iter().filter(|e| matches!(e, ChunkEvent::Final { .. })).collect();
        assert_eq!(finals.len(), 1);
        if let ChunkEvent::Final { pcm, start_ms, end_ms } = finals[0] {
            assert_eq!(*start_ms, 0);
            assert!(*end_ms >= 1000 && *end_ms <= 1700, "end_ms {end_ms}");
            assert!(pcm.len() >= TARGET_RATE as usize);
        }
    }

    #[test]
    fn chunker_emits_partials_every_two_seconds_and_caps_at_eight() {
        let mut c = Chunker::new();
        let ev = c.push(&sine(TARGET_RATE, 9.0, 0.3));
        let partials = ev.iter().filter(|e| matches!(e, ChunkEvent::Partial { .. })).count();
        let finals = ev.iter().filter(|e| matches!(e, ChunkEvent::Final { .. })).count();
        assert!(partials >= 3, "partials {partials}");
        assert_eq!(finals, 1, "8s cap must produce exactly one Final");
    }

    #[test]
    fn chunker_drops_leading_silence_and_tracks_offsets() {
        let mut c = Chunker::new();
        let mut ev = c.push(&silence(2.0));
        assert!(ev.is_empty());
        ev.extend(c.push(&sine(TARGET_RATE, 0.5, 0.3)));
        ev.extend(c.push(&silence(0.7)));
        let f = ev.iter().find(|e| matches!(e, ChunkEvent::Final { .. })).expect("final");
        if let ChunkEvent::Final { start_ms, .. } = f {
            assert!((1900..=2100).contains(start_ms), "start_ms {start_ms}");
        }
    }

    #[test]
    fn flush_returns_pending_speech() {
        let mut c = Chunker::new();
        c.push(&sine(TARGET_RATE, 0.5, 0.3));
        assert!(matches!(c.flush(), Some(ChunkEvent::Final { .. })));
        assert!(c.flush().is_none());
    }
}
```

- [ ] **Step 2: 실패 확인**

`lib.rs`에 `pub mod audio;` 추가 후 `cargo test -p babelay-engine audio` → 컴파일 실패.

- [ ] **Step 3: 구현**

```rust
//! 16kHz 모노 변환과 음성 조각화.
pub const TARGET_RATE: u32 = 16_000;
const FRAME: usize = TARGET_RATE as usize / 50; // 20ms
const RMS_THRESHOLD: f32 = 0.01;
const SILENCE_END: usize = TARGET_RATE as usize * 6 / 10; // 0.6s
const MAX_CHUNK: usize = TARGET_RATE as usize * 8;
const PARTIAL_EVERY: usize = TARGET_RATE as usize * 2;
const DROP_SILENCE: usize = TARGET_RATE as usize; // 1s

pub struct Resampler {
    src_rate: u32,
    channels: u16,
    pos: f64,       // 다음 출력 샘플의 소스 위치(모노 샘플 단위)
    last: f32,      // 직전 소스 모노 샘플(경계 보간용)
    have_last: bool,
}

impl Resampler {
    pub fn new(src_rate: u32, channels: u16) -> Self {
        Self { src_rate, channels: channels.max(1), pos: 0.0, last: 0.0, have_last: false }
    }

    /// 인터리브 입력을 모노 16kHz 로 변환해 `out` 에 덧붙인다.
    pub fn push(&mut self, interleaved: &[f32], out: &mut Vec<f32>) {
        let ch = self.channels as usize;
        let mono: Vec<f32> = interleaved.chunks_exact(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect();
        if mono.is_empty() {
            return;
        }
        let step = self.src_rate as f64 / TARGET_RATE as f64;
        // 소스 인덱스 -1 은 직전 블록의 마지막 샘플
        let at = |i: i64, last: f32, mono: &[f32]| -> f32 { if i < 0 { last } else { mono[i as usize] } };
        let base = if self.have_last { -1.0 } else { 0.0 };
        let mut pos = self.pos + base; // 이번 블록 좌표계(-1 = last)
        let n = mono.len() as f64;
        while pos + 1.0 < n {
            let i = pos.floor() as i64;
            let frac = (pos - pos.floor()) as f32;
            let a = at(i, self.last, &mono);
            let b = at(i + 1, self.last, &mono);
            out.push(a + (b - a) * frac);
            pos += step;
        }
        self.pos = pos - (n - 1.0); // 다음 블록에서 last 가 -1 이 되도록
        self.last = *mono.last().unwrap();
        self.have_last = true;
    }
}

pub enum ChunkEvent {
    Partial { pcm: Vec<f32>, start_ms: u64 },
    Final { pcm: Vec<f32>, start_ms: u64, end_ms: u64 },
}

pub struct Chunker {
    buf: Vec<f32>,
    consumed: u64,      // 세션 시작부터 버퍼 앞까지 흘려보낸 샘플 수
    speech_seen: bool,
    silence_run: usize,
    since_partial: usize,
    pending: Vec<f32>,  // 20ms 프레임 미만 잔여
}

impl Default for Chunker {
    fn default() -> Self { Self::new() }
}

impl Chunker {
    pub fn new() -> Self {
        Self { buf: Vec::new(), consumed: 0, speech_seen: false, silence_run: 0, since_partial: 0, pending: Vec::new() }
    }

    fn ms(samples: u64) -> u64 { samples * 1000 / TARGET_RATE as u64 }

    pub fn push(&mut self, mono16k: &[f32]) -> Vec<ChunkEvent> {
        let mut events = Vec::new();
        let mut data = std::mem::take(&mut self.pending);
        data.extend_from_slice(mono16k);
        let mut frames = data.chunks_exact(FRAME);
        for frame in &mut frames {
            let rms = (frame.iter().map(|s| s * s).sum::<f32>() / FRAME as f32).sqrt();
            let speech = rms > RMS_THRESHOLD;
            self.buf.extend_from_slice(frame);
            self.since_partial += FRAME;
            if speech {
                self.speech_seen = true;
                self.silence_run = 0;
            } else {
                self.silence_run += FRAME;
            }
            if !self.speech_seen {
                if self.buf.len() >= DROP_SILENCE {
                    self.consumed += self.buf.len() as u64;
                    self.buf.clear();
                    self.silence_run = 0;
                    self.since_partial = 0;
                }
                continue;
            }
            if self.silence_run >= SILENCE_END || self.buf.len() >= MAX_CHUNK {
                events.push(self.finalize());
            } else if self.since_partial >= PARTIAL_EVERY {
                self.since_partial = 0;
                events.push(ChunkEvent::Partial { pcm: self.buf.clone(), start_ms: Self::ms(self.consumed) });
            }
        }
        self.pending = frames.remainder().to_vec();
        events
    }

    fn finalize(&mut self) -> ChunkEvent {
        let pcm = std::mem::take(&mut self.buf);
        let start_ms = Self::ms(self.consumed);
        self.consumed += pcm.len() as u64;
        let end_ms = Self::ms(self.consumed);
        self.speech_seen = false;
        self.silence_run = 0;
        self.since_partial = 0;
        ChunkEvent::Final { pcm, start_ms, end_ms }
    }

    pub fn flush(&mut self) -> Option<ChunkEvent> {
        if self.speech_seen && !self.buf.is_empty() { Some(self.finalize()) } else { None }
    }
}
```

- [ ] **Step 4: 통과 확인**

```bash
mise exec -- cargo test -p babelay-engine audio
mise exec -- cargo clippy -p babelay-engine --all-targets -- -D warnings
```

Expected: 6 PASS. 리샘플러 경계 로직에서 테스트가 ±2 샘플을 벗어나면 `while pos + 1.0 < n` 조건과 `self.pos` 이월 계산을 맞춘다(출력 개수 = floor(입력/step)).

- [ ] **Step 5: Commit** — `feat(engine): linear resampler and energy-VAD chunker`

---

### Task 2: macOS 캡처 (Core Audio Process Tap, ObjC 심)

**Files:**
- Create: `crates/babelay-engine/csrc/tap.m`, `crates/babelay-engine/build.rs`, `crates/babelay-engine/src/capture/mod.rs`, `crates/babelay-engine/src/capture/macos.rs`
- Modify: `crates/babelay-engine/Cargo.toml`, `crates/babelay-engine/src/lib.rs`, `.mise.toml`

**Interfaces:**
- Produces:
  ```rust
  // capture/mod.rs
  pub struct Frame { pub samples: Vec<f32>, pub rate: u32, pub channels: u16 }   // 인터리브
  pub type Sink = Box<dyn FnMut(Frame) + Send + 'static>;
  pub trait AudioSource: Send { fn start(&mut self, sink: Sink) -> Result<(), CaptureError>; fn stop(&mut self); }
  #[derive(thiserror::Error, Debug)] pub enum CaptureError { #[error("permission denied")] Permission, #[error("no output device")] NoDevice, #[error("os error {0}")] Os(i32), #[error("{0}")] Other(String) }
  pub fn default_source() -> Box<dyn AudioSource>;
  pub fn probe_permission() -> Permission;   // enum Permission { Granted, Denied, Unknown }
  ```
  C ABI (`tap.m`): `int babelay_tap_start(void (*cb)(const float*, uint32_t frames, uint32_t channels, double rate, void* user), void* user, void** handle_out)`, `void babelay_tap_stop(void* handle)`, `int babelay_tap_probe(void)`(0 = granted, 1 = denied, 2 = unknown).

- [ ] **Step 1: cmake와 툴체인**

`.mise.toml`의 `[tools]`에 `cmake = "4"`를 추가하고 `mise install`. `mise exec -- cmake --version`이 4.x를 출력해야 한다(whisper.cpp 빌드용, Task 4에서 필요하지만 여기서 미리).

- [ ] **Step 2: Cargo.toml / build.rs**

`crates/babelay-engine/Cargo.toml`에 추가:

```toml
[build-dependencies]
cc = "1"

[target.'cfg(target_os = "macos")'.dependencies]
# ObjC 심은 cc 로 링크; 추가 크레이트 없음
```

`build.rs`:

```rust
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("csrc/tap.m")
            .flag("-fobjc-arc")
            .flag("-fmodules")
            .compile("babelay_tap");
        println!("cargo:rustc-link-lib=framework=CoreAudio");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rerun-if-changed=csrc/tap.m");
    }
}
```

- [ ] **Step 3: ObjC 심**

`csrc/tap.m`:

```objc
#import <Foundation/Foundation.h>
#import <CoreAudio/CoreAudio.h>
#import <CoreAudio/AudioHardwareTapping.h>
#import <CoreAudio/CATapDescription.h>

typedef void (*babelay_cb)(const float*, uint32_t, uint32_t, double, void*);

typedef struct {
    AudioObjectID tap;
    AudioObjectID aggregate;
    AudioDeviceIOProcID proc;
    babelay_cb cb;
    void* user;
    uint32_t channels;
    double rate;
} tap_handle;

static OSStatus default_output_uid(NSString** uid) {
    AudioObjectPropertyAddress addr = { kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain };
    AudioObjectID dev = 0; UInt32 size = sizeof(dev);
    OSStatus st = AudioObjectGetPropertyData(kAudioObjectSystemObject, &addr, 0, NULL, &size, &dev);
    if (st != noErr) return st;
    addr.mSelector = kAudioDevicePropertyDeviceUID;
    CFStringRef cf = NULL; size = sizeof(cf);
    st = AudioObjectGetPropertyData(dev, &addr, 0, NULL, &size, &cf);
    if (st != noErr) return st;
    *uid = (__bridge_transfer NSString*)cf;
    return noErr;
}

static OSStatus create_tap(AudioObjectID* tapOut, CATapDescription** descOut) {
    CATapDescription* desc = [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:@[]];
    desc.name = @"Babelay";
    desc.privateTap = YES;
    desc.muteBehavior = CATapUnmuted;
    OSStatus st = AudioHardwareCreateProcessTap(desc, tapOut);
    if (st == noErr && descOut) *descOut = desc;
    return st;
}

int babelay_tap_probe(void) {
    AudioObjectID tap = 0;
    OSStatus st = create_tap(&tap, NULL);
    if (st == noErr) { AudioHardwareDestroyProcessTap(tap); return 0; }
    return 1; // 탭 생성 실패 = 권한 거부(또는 미지원). 14.2 미만은 링크 단계에서 걸러진다.
}

int babelay_tap_start(babelay_cb cb, void* user, void** handle_out) {
    tap_handle* h = calloc(1, sizeof(tap_handle));
    h->cb = cb; h->user = user;
    CATapDescription* desc = NULL;
    OSStatus st = create_tap(&h->tap, &desc);
    if (st != noErr) { free(h); return (int)st; }

    NSString* outUID = nil;
    st = default_output_uid(&outUID);
    if (st != noErr) { AudioHardwareDestroyProcessTap(h->tap); free(h); return (int)st; }

    NSDictionary* aggDesc = @{
        @(kAudioAggregateDeviceNameKey): @"Babelay Tap",
        @(kAudioAggregateDeviceUIDKey): [NSString stringWithFormat:@"com.babelay.tap.%@", desc.UUID.UUIDString],
        @(kAudioAggregateDeviceIsPrivateKey): @YES,
        @(kAudioAggregateDeviceIsStackedKey): @NO,
        @(kAudioAggregateDeviceTapAutoStartKey): @YES,
        @(kAudioAggregateDeviceSubDeviceListKey): @[ @{ @(kAudioSubDeviceUIDKey): outUID } ],
        @(kAudioAggregateDeviceTapListKey): @[ @{ @(kAudioSubTapDriftCompensationKey): @YES, @(kAudioSubTapUIDKey): desc.UUID.UUIDString } ],
    };
    st = AudioHardwareCreateAggregateDevice((__bridge CFDictionaryRef)aggDesc, &h->aggregate);
    if (st != noErr) { AudioHardwareDestroyProcessTap(h->tap); free(h); return (int)st; }

    AudioObjectPropertyAddress fmtAddr = { kAudioTapPropertyFormat, kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain };
    AudioStreamBasicDescription asbd = {0}; UInt32 size = sizeof(asbd);
    st = AudioObjectGetPropertyData(h->tap, &fmtAddr, 0, NULL, &size, &asbd);
    if (st != noErr) { babelay_tap_stop(h); return (int)st; }
    h->channels = asbd.mChannelsPerFrame; h->rate = asbd.mSampleRate;

    st = AudioDeviceCreateIOProcIDWithBlock(&h->proc, h->aggregate, NULL,
        ^(const AudioTimeStamp* now, const AudioBufferList* input, const AudioTimeStamp* inTime,
          AudioBufferList* output, const AudioTimeStamp* outTime) {
            for (UInt32 i = 0; i < input->mNumberBuffers; i++) {
                const AudioBuffer* b = &input->mBuffers[i];
                uint32_t ch = b->mNumberChannels ? b->mNumberChannels : h->channels;
                uint32_t frames = b->mDataByteSize / (sizeof(float) * ch);
                if (frames) h->cb((const float*)b->mData, frames, ch, h->rate, h->user);
            }
        });
    if (st != noErr) { babelay_tap_stop(h); return (int)st; }
    st = AudioDeviceStart(h->aggregate, h->proc);
    if (st != noErr) { babelay_tap_stop(h); return (int)st; }
    *handle_out = h;
    return 0;
}

void babelay_tap_stop(void* handle) {
    tap_handle* h = (tap_handle*)handle;
    if (!h) return;
    if (h->proc) { AudioDeviceStop(h->aggregate, h->proc); AudioDeviceDestroyIOProcID(h->aggregate, h->proc); }
    if (h->aggregate) AudioHardwareDestroyAggregateDevice(h->aggregate);
    if (h->tap) AudioHardwareDestroyProcessTap(h->tap);
    free(h);
}
```

컴파일 오류가 나면 헤더 이름(`AudioHardwareTapping.h`, `CATapDescription.h`)과 키 상수 이름을 `xcrun --show-sdk-path`의 `CoreAudio.framework/Headers`에서 확인해 맞춘다. 탭 포맷이 float32 인터리브가 아니면(`mFormatFlags`에 `kAudioFormatFlagIsNonInterleaved`) 버퍼가 채널별로 오므로 `mNumberBuffers`개 버퍼를 인터리브해 콜백에 넘긴다.

- [ ] **Step 4: Rust 바인딩**

`capture/mod.rs`:

```rust
pub struct Frame { pub samples: Vec<f32>, pub rate: u32, pub channels: u16 }
pub type Sink = Box<dyn FnMut(Frame) + Send + 'static>;

pub trait AudioSource: Send {
    fn start(&mut self, sink: Sink) -> Result<(), CaptureError>;
    fn stop(&mut self);
}

#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("permission denied")] Permission,
    #[error("no output device")] NoDevice,
    #[error("os error {0}")] Os(i32),
    #[error("{0}")] Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission { Granted, Denied, Unknown }

#[cfg(target_os = "macos")] pub mod macos;
#[cfg(target_os = "windows")] pub mod windows;

pub fn default_source() -> Box<dyn AudioSource> {
    #[cfg(target_os = "macos")] { Box::new(macos::TapSource::default()) }
    #[cfg(target_os = "windows")] { Box::new(windows::LoopbackSource::default()) }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))] { Box::new(Unsupported) }
}

pub fn probe_permission() -> Permission {
    #[cfg(target_os = "macos")] { macos::probe() }
    #[cfg(target_os = "windows")] { Permission::Granted }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))] { Permission::Unknown }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
struct Unsupported;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl AudioSource for Unsupported {
    fn start(&mut self, _: Sink) -> Result<(), CaptureError> { Err(CaptureError::Other("unsupported platform".into())) }
    fn stop(&mut self) {}
}
```

`capture/macos.rs`:

```rust
use super::{AudioSource, CaptureError, Frame, Permission, Sink};
use std::ffi::c_void;

type Cb = unsafe extern "C" fn(*const f32, u32, u32, f64, *mut c_void);
extern "C" {
    fn babelay_tap_start(cb: Cb, user: *mut c_void, handle_out: *mut *mut c_void) -> i32;
    fn babelay_tap_stop(handle: *mut c_void);
    fn babelay_tap_probe() -> i32;
}

pub struct TapSource { handle: *mut c_void, sink: Option<Box<Sink>> }
impl Default for TapSource { fn default() -> Self { Self { handle: std::ptr::null_mut(), sink: None } } }
unsafe impl Send for TapSource {}

unsafe extern "C" fn trampoline(data: *const f32, frames: u32, channels: u32, rate: f64, user: *mut c_void) {
    let sink = &mut *(user as *mut Sink);
    let n = frames as usize * channels as usize;
    let samples = std::slice::from_raw_parts(data, n).to_vec();
    sink(Frame { samples, rate: rate as u32, channels: channels as u16 });
}

impl AudioSource for TapSource {
    fn start(&mut self, sink: Sink) -> Result<(), CaptureError> {
        let boxed = Box::new(sink);
        let user = Box::into_raw(boxed);
        let mut handle = std::ptr::null_mut();
        let st = unsafe { babelay_tap_start(trampoline, user as *mut c_void, &mut handle) };
        if st != 0 {
            unsafe { drop(Box::from_raw(user)) };
            return Err(if st == 0x6e6f7065 /* 'nope' */ { CaptureError::Permission } else { CaptureError::Os(st) });
        }
        self.handle = handle;
        self.sink = Some(unsafe { Box::from_raw(user) }); // 소유권 회수: stop 전까지 살아있음
        // 위 한 줄은 콜백이 같은 포인터를 계속 쓰므로, stop() 이 babelay_tap_stop 후에야 drop 되게 순서를 지킨다.
        Ok(())
    }
    fn stop(&mut self) {
        if !self.handle.is_null() {
            unsafe { babelay_tap_stop(self.handle) };
            self.handle = std::ptr::null_mut();
        }
        self.sink = None;
    }
}
impl Drop for TapSource { fn drop(&mut self) { self.stop(); } }

pub fn probe() -> Permission {
    match unsafe { babelay_tap_probe() } { 0 => Permission::Granted, 1 => Permission::Denied, _ => Permission::Unknown }
}
```

주의: `self.sink = Some(Box::from_raw(user))`로 소유권을 회수하면 Rust가 `Box<Sink>`를 들고 있고, C 콜백은 같은 힙 주소를 쓴다. `stop()`에서 `babelay_tap_stop`(IOProc 정지·파괴) 뒤에 `sink = None`이 되므로 안전하다. `TapSource`는 이동될 수 있어도 `Box` 내부 주소는 불변이다.

- [ ] **Step 5: 수동 검증 테스트(#[ignore])**

`capture/macos.rs` 하단:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    #[ignore = "needs system audio permission; run with --ignored while audio plays"]
    fn captures_some_frames() {
        let got = Arc::new(Mutex::new(0usize));
        let g = got.clone();
        let mut src = TapSource::default();
        src.start(Box::new(move |f: Frame| { *g.lock().unwrap() += f.samples.len(); })).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        src.stop();
        assert!(*got.lock().unwrap() > 0);
    }
}
```

- [ ] **Step 6: 게이트**

`lib.rs`에 `pub mod capture;`. `mise exec -- cargo test -p babelay-engine`(ignored 제외 통과), clippy, 그리고 실제 확인: `mise exec -- cargo test -p babelay-engine captures_some_frames -- --ignored --nocapture`를 음악을 재생하며 실행. 첫 실행에 macOS가 "시스템 오디오 녹음" 권한을 묻는다(터미널 앱에 대해). 거부/무응답이면 테스트 실패가 정상이며 보고서에 적는다.

- [ ] **Step 7: Commit** — `feat(engine): macOS system audio capture via Core Audio process tap`

---

### Task 3: Windows 캡처 (WASAPI 루프백)

**Files:**
- Create: `crates/babelay-engine/src/capture/windows.rs`
- Modify: `crates/babelay-engine/Cargo.toml`

**Interfaces:**
- Produces: `windows::LoopbackSource: AudioSource`(기본 출력 장치 루프백, 이벤트 구동, 스레드 하나)

- [ ] **Step 1: 의존성과 타깃**

```toml
[target.'cfg(target_os = "windows")'.dependencies]
wasapi = "0.24"
```

`rustup target add x86_64-pc-windows-msvc`(std만 설치; 링크는 하지 않는다).

- [ ] **Step 2: 구현**

```rust
use super::{AudioSource, CaptureError, Frame, Sink};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

#[derive(Default)]
pub struct LoopbackSource { stop: Option<Arc<AtomicBool>>, thread: Option<std::thread::JoinHandle<()>> }

impl AudioSource for LoopbackSource {
    fn start(&mut self, mut sink: Sink) -> Result<(), CaptureError> {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        let thread = std::thread::spawn(move || {
            let run = || -> Result<(), String> {
                wasapi::initialize_mta().ok().ok_or("COM init")?;
                let device = DeviceEnumerator::new().map_err(|e| e.to_string())?
                    .get_default_device(&Direction::Render).map_err(|e| e.to_string())?;
                let mut client = device.get_iaudioclient().map_err(|e| e.to_string())?;
                let mix = client.get_mixformat().map_err(|e| e.to_string())?;
                let rate = mix.get_samplespersec();
                let channels = mix.get_nchannels();
                let fmt = WaveFormat::new(32, 32, &SampleType::Float, rate as usize, channels as usize, None);
                client.initialize_client(&fmt, &Direction::Capture, &StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: 200_000 })
                    .map_err(|e| e.to_string())?;
                let event = client.set_get_eventhandle().map_err(|e| e.to_string())?;
                let capture = client.get_audiocaptureclient().map_err(|e| e.to_string())?;
                client.start_stream().map_err(|e| e.to_string())?;
                let _ = ready_tx.send(Ok(()));
                let mut bytes = std::collections::VecDeque::new();
                while !stop2.load(Ordering::Relaxed) {
                    if event.wait_for_event(1000).is_err() { continue; }
                    capture.read_from_device_to_deque(&mut bytes).map_err(|e| e.to_string())?;
                    let n = bytes.len() / 4;
                    if n == 0 { continue; }
                    let mut samples = Vec::with_capacity(n);
                    for _ in 0..n {
                        let b = [bytes.pop_front().unwrap(), bytes.pop_front().unwrap(), bytes.pop_front().unwrap(), bytes.pop_front().unwrap()];
                        samples.push(f32::from_le_bytes(b));
                    }
                    sink(Frame { samples, rate, channels });
                }
                let _ = client.stop_stream();
                Ok(())
            };
            if let Err(e) = run() { let _ = ready_tx.send(Err(e)); }
        });
        match ready_rx.recv() {
            Ok(Ok(())) => { self.stop = Some(stop); self.thread = Some(thread); Ok(()) }
            Ok(Err(e)) => Err(CaptureError::Other(e)),
            Err(_) => Err(CaptureError::Other("capture thread died".into())),
        }
    }
    fn stop(&mut self) {
        if let Some(s) = self.stop.take() { s.store(true, Ordering::Relaxed); }
        if let Some(t) = self.thread.take() { let _ = t.join(); }
    }
}
impl Drop for LoopbackSource { fn drop(&mut self) { self.stop(); } }
```

`wasapi::initialize_mta` 이름이 다르면(예: `initialize_com`) 크레이트 소스에서 찾아 맞춘다. `get_mixformat`이 `AudioClient`가 아니라 `Device`에 있으면 그에 맞게 호출한다.

- [ ] **Step 3: 게이트**

```bash
mise exec -- cargo check -p babelay-engine --target x86_64-pc-windows-msvc
mise exec -- cargo test -p babelay-engine
```

Expected: Windows 타깃 check 통과(링크 없음), macOS 테스트 그대로.

- [ ] **Step 4: Commit** — `feat(engine): windows WASAPI loopback capture`

---

### Task 4: Whisper 전사기

**Files:**
- Create: `crates/babelay-engine/src/transcribe.rs`
- Modify: `crates/babelay-engine/Cargo.toml`, `lib.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Debug, serde::Serialize)] pub struct Segment { pub text: String, pub lang: String, pub t0_ms: u64, pub t1_ms: u64 }
  pub trait Transcriber: Send { fn transcribe(&mut self, pcm16k: &[f32], lang: Option<&str>) -> Result<Vec<Segment>, TranscribeError>; }
  pub struct WhisperTranscriber { /* ctx, state, threads */ pub gpu_active: bool }
  impl WhisperTranscriber { pub fn load(model: &Path, use_gpu: bool) -> Result<(Self, bool /*fell_back*/), TranscribeError>; }
  ```
  `lang`: `None` = 자동 감지. 조각 텍스트는 세그먼트를 공백으로 이어 붙이고(빈 것은 제외) 하나의 `Segment`로 돌려준다(휘스퍼 내부 세그먼트 분할은 자막에 불필요). `t0/t1`은 호출자가 채우므로 0으로 두고, `lang`은 `full_lang_id_from_state` → `get_lang_str`.

- [ ] **Step 1: 의존성**

```toml
[dependencies]
whisper-rs = "0.16"

[features]
metal = ["whisper-rs/metal"]
cuda = ["whisper-rs/cuda"]
```

- [ ] **Step 2: 실패하는 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_segments_skips_blank_and_trims() {
        assert_eq!(join(&["  Hello", "", " world. "]), "Hello world.");
    }

    #[test]
    #[ignore = "needs BABELAY_TEST_MODEL=path/to/ggml-tiny.bin"]
    fn transcribes_synthetic_silence_without_panicking() {
        let path = std::env::var("BABELAY_TEST_MODEL").expect("BABELAY_TEST_MODEL");
        let (mut t, _) = WhisperTranscriber::load(std::path::Path::new(&path), true).unwrap();
        let pcm = vec![0.0f32; 16_000];
        let segs = t.transcribe(&pcm, Some("en")).unwrap();
        assert!(segs.len() <= 1);
    }
}
```

- [ ] **Step 3: 구현**

```rust
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

#[derive(Clone, Debug, serde::Serialize)]
pub struct Segment { pub text: String, pub lang: String, pub t0_ms: u64, pub t1_ms: u64 }

#[derive(thiserror::Error, Debug)]
pub enum TranscribeError {
    #[error("model load failed: {0}")] Load(String),
    #[error("inference failed: {0}")] Inference(String),
}

pub trait Transcriber: Send {
    fn transcribe(&mut self, pcm16k: &[f32], lang: Option<&str>) -> Result<Vec<Segment>, TranscribeError>;
}

pub struct WhisperTranscriber { ctx: WhisperContext, state: WhisperState, threads: i32, pub gpu_active: bool }

impl WhisperTranscriber {
    pub fn load(model: &Path, use_gpu: bool) -> Result<(Self, bool), TranscribeError> {
        let path = model.to_str().ok_or_else(|| TranscribeError::Load("non-utf8 path".into()))?;
        let make = |gpu: bool| {
            let mut p = WhisperContextParameters::default();
            p.use_gpu(gpu);
            WhisperContext::new_with_params(path, p)
        };
        let (ctx, fell_back) = match make(use_gpu) {
            Ok(c) => (c, false),
            Err(e) if use_gpu => (make(false).map_err(|e2| TranscribeError::Load(format!("{e}; cpu: {e2}")))?, true),
            Err(e) => return Err(TranscribeError::Load(e.to_string())),
        };
        let state = ctx.create_state().map_err(|e| TranscribeError::Load(e.to_string()))?;
        let threads = std::thread::available_parallelism().map(|n| n.get() as i32).unwrap_or(4).min(8);
        Ok((Self { ctx, state, threads, gpu_active: use_gpu && !fell_back }, fell_back))
    }
}

pub(crate) fn join(parts: &[&str]) -> String {
    parts.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
}

impl Transcriber for WhisperTranscriber {
    fn transcribe(&mut self, pcm16k: &[f32], lang: Option<&str>) -> Result<Vec<Segment>, TranscribeError> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(lang.unwrap_or("auto")));
        params.set_n_threads(self.threads);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_suppress_blank(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        self.state.full(params, pcm16k).map_err(|e| TranscribeError::Inference(e.to_string()))?;
        let n = self.state.full_n_segments();
        let mut parts = Vec::new();
        for i in 0..n {
            if let Some(seg) = self.state.get_segment(i) {
                if let Ok(s) = seg.to_str_lossy() { parts.push(s.into_owned()); }
            }
        }
        let text = join(&parts.iter().map(String::as_str).collect::<Vec<_>>());
        if text.is_empty() { return Ok(vec![]); }
        let lang_id = self.state.full_lang_id_from_state();
        let lang = whisper_rs::get_lang_str(lang_id).unwrap_or("en").to_string();
        let _ = &self.ctx;
        Ok(vec![Segment { text, lang, t0_ms: 0, t1_ms: 0 }])
    }
}
```

`WhisperState`가 `WhisperContext`를 빌리는 라이프타임 구조라면(0.16에서 `create_state`가 `Result<WhisperState, _>`로 소유형이면 그대로), 컴파일 오류 시 `ctx`를 `Arc`로 감싸거나 `state`를 호출마다 만들도록 바꾸고 보고한다. `full_n_segments`/`get_segment`가 `WhisperState`에 없고 `as_iter()`만 있으면 `for seg in self.state.as_iter()`로 바꾼다.

- [ ] **Step 4: 빌드와 게이트**

`src-tauri/Cargo.toml`은 아직 건드리지 않는다. 엔진만:

```bash
mise exec -- cargo test -p babelay-engine --features metal
mise exec -- cargo clippy -p babelay-engine --all-targets --features metal -- -D warnings
```

whisper.cpp 첫 빌드는 수 분 걸린다. cmake를 못 찾으면 `.mise.toml`/`mise install`을 확인한다. 실제 모델 테스트: `mise exec -- yarn tauri dev`로 앱을 켜 Whisper Tiny를 받은 뒤(`~/Library/Application Support/com.babelay.app/models/asr/ggml-tiny.bin`), `BABELAY_TEST_MODEL=<그 경로> mise exec -- cargo test -p babelay-engine --features metal transcribes_synthetic -- --ignored`.

- [ ] **Step 5: Commit** — `feat(engine): whisper transcriber with GPU fallback`

---

### Task 5: 엔진 오케스트레이션

**Files:**
- Create: `crates/babelay-engine/src/engine.rs`
- Modify: `lib.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct EngineConfig { pub model_path: PathBuf, pub use_gpu: bool, pub source_lang: Option<String> /* None = auto */ }
  #[derive(Clone, Debug, serde::Serialize)] #[serde(tag = "type", rename_all = "snake_case")]
  pub enum EngineEvent {
      Started { gpu_active: bool, gpu_fallback: bool },
      Partial { text: String, lang: String, start_ms: u64 },
      Final { id: u64, text: String, lang: String, start_ms: u64, end_ms: u64 },
      Lagging { queued_ms: u64 },
      Error { code: String, message: String },
      Stopped,
  }
  pub struct EngineHandle { /* stop flag, join handles */ }
  impl EngineHandle { pub fn stop(self); }
  pub fn start(cfg: EngineConfig, source: Box<dyn AudioSource>, transcriber: Box<dyn Transcriber>, gpu_fallback: bool, tx: std::sync::mpsc::Sender<EngineEvent>) -> Result<EngineHandle, String>;
  pub fn start_default(cfg: EngineConfig, tx: Sender<EngineEvent>) -> Result<EngineHandle, String>;   // default_source + WhisperTranscriber::load
  ```
  스레드: 캡처 콜백 → `frames` 채널(unbounded) → 청커 스레드(리샘플+Chunker) → `chunks` 채널(bounded 8; Partial은 `try_send`로 버림, Final은 blocking) → 전사 스레드 → `tx`. `Final` id는 1부터 증가. 전사 스레드는 조각의 `enqueued_at`이 10s를 넘으면 `Lagging` 1회 발행(다시 정상이 되면 리셋). `stop()`은 캡처 정지 → 청커 `flush` → 채널 닫기 → 스레드 join → `Stopped`.

- [ ] **Step 1: 실패하는 테스트(가짜 소스·전사기)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{AudioSource, CaptureError, Frame, Sink};
    use crate::transcribe::{Segment, Transcriber, TranscribeError};
    use std::sync::mpsc;
    use std::time::Duration;

    struct FakeSource { stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>> }
    impl AudioSource for FakeSource {
        fn start(&mut self, mut sink: Sink) -> Result<(), CaptureError> {
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let s = stop.clone();
            std::thread::spawn(move || {
                // 1.5s 톤 + 1s 무음 을 48kHz 스테레오로 20ms 단위 전송
                let mut t = 0usize;
                while !s.load(std::sync::atomic::Ordering::Relaxed) && t < 125 {
                    let amp = if t < 75 { 0.3 } else { 0.0 };
                    let frame: Vec<f32> = (0..960 * 2).map(|i| amp * ((i / 2) as f32 * 0.1).sin()).collect();
                    sink(Frame { samples: frame, rate: 48_000, channels: 2 });
                    t += 1;
                    std::thread::sleep(Duration::from_millis(2));
                }
            });
            self.stop = Some(stop);
            Ok(())
        }
        fn stop(&mut self) { if let Some(s) = &self.stop { s.store(true, std::sync::atomic::Ordering::Relaxed); } }
    }

    struct FakeTranscriber;
    impl Transcriber for FakeTranscriber {
        fn transcribe(&mut self, pcm: &[f32], _lang: Option<&str>) -> Result<Vec<Segment>, TranscribeError> {
            Ok(vec![Segment { text: format!("{} samples", pcm.len()), lang: "en".into(), t0_ms: 0, t1_ms: 0 }])
        }
    }

    #[test]
    fn pipeline_emits_started_final_and_stopped() {
        let (tx, rx) = mpsc::channel();
        let cfg = EngineConfig { model_path: "unused".into(), use_gpu: false, source_lang: None };
        let handle = start(cfg, Box::new(FakeSource { stop: None }), Box::new(FakeTranscriber), false, tx).unwrap();
        let mut events = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Ok(e) = rx.recv_timeout(Duration::from_millis(200)) { events.push(e); }
            if events.iter().any(|e| matches!(e, EngineEvent::Final { .. })) { break; }
        }
        handle.stop();
        while let Ok(e) = rx.recv_timeout(Duration::from_secs(2)) { events.push(e); if matches!(e, EngineEvent::Stopped) { break; } }
        assert!(matches!(events[0], EngineEvent::Started { .. }));
        let f = events.iter().find(|e| matches!(e, EngineEvent::Final { .. })).expect("a Final event");
        if let EngineEvent::Final { id, text, start_ms, end_ms, .. } = f {
            assert_eq!(*id, 1);
            assert!(text.ends_with("samples"));
            assert!(end_ms > start_ms);
        }
        assert!(matches!(events.last(), Some(EngineEvent::Stopped)));
    }
}
```

- [ ] **Step 2: 구현**

```rust
use crate::audio::{ChunkEvent, Chunker, Resampler, TARGET_RATE};
use crate::capture::{default_source, AudioSource, Frame};
use crate::transcribe::{Transcriber, WhisperTranscriber};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct EngineConfig { pub model_path: PathBuf, pub use_gpu: bool, pub source_lang: Option<String> }

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    Started { gpu_active: bool, gpu_fallback: bool },
    Partial { text: String, lang: String, start_ms: u64 },
    Final { id: u64, text: String, lang: String, start_ms: u64, end_ms: u64 },
    Lagging { queued_ms: u64 },
    Error { code: String, message: String },
    Stopped,
}

struct Job { ev: ChunkEvent, enqueued: Instant }

pub struct EngineHandle {
    source: Box<dyn AudioSource>,
    frames_tx: Option<Sender<Frame>>,
    chunker: Option<JoinHandle<()>>,
    transcriber: Option<JoinHandle<()>>,
}

impl EngineHandle {
    pub fn stop(mut self) {
        self.source.stop();
        drop(self.frames_tx.take());              // 청커 스레드가 flush 후 종료
        if let Some(h) = self.chunker.take() { let _ = h.join(); }
        if let Some(h) = self.transcriber.take() { let _ = h.join(); }
    }
}

pub fn start_default(cfg: EngineConfig, tx: Sender<EngineEvent>) -> Result<EngineHandle, String> {
    let (t, fell_back) = WhisperTranscriber::load(&cfg.model_path, cfg.use_gpu).map_err(|e| e.to_string())?;
    start(cfg, default_source(), Box::new(t), fell_back, tx)
}

pub fn start(cfg: EngineConfig, mut source: Box<dyn AudioSource>, transcriber: Box<dyn Transcriber>, gpu_fallback: bool, tx: Sender<EngineEvent>) -> Result<EngineHandle, String> {
    let (frames_tx, frames_rx) = mpsc::channel::<Frame>();
    let (chunks_tx, chunks_rx) = mpsc::sync_channel::<Job>(8);

    let chunker = std::thread::spawn(move || chunker_loop(frames_rx, chunks_tx));
    let tx2 = tx.clone();
    let lang = cfg.source_lang.clone();
    let transcriber_thread = std::thread::spawn(move || {
        let mut transcriber = transcriber;
        transcribe_loop(chunks_rx, &mut *transcriber, lang.as_deref(), tx2)
    });

    let ftx = frames_tx.clone();
    source.start(Box::new(move |f| { let _ = ftx.send(f); })).map_err(|e| e.to_string())?;
    let _ = tx.send(EngineEvent::Started { gpu_active: cfg.use_gpu && !gpu_fallback, gpu_fallback });
    Ok(EngineHandle { source, frames_tx: Some(frames_tx), chunker: Some(chunker), transcriber: Some(transcriber_thread) })
}

fn chunker_loop(rx: Receiver<Frame>, tx: SyncSender<Job>) {
    let mut resampler: Option<Resampler> = None;
    let mut chunker = Chunker::new();
    let mut mono = Vec::new();
    for f in rx {
        let r = resampler.get_or_insert_with(|| Resampler::new(f.rate, f.channels));
        mono.clear();
        r.push(&f.samples, &mut mono);
        for ev in chunker.push(&mono) {
            let job = Job { ev, enqueued: Instant::now() };
            match job.ev {
                ChunkEvent::Partial { .. } => { let _ = tx.try_send(job); } // 큐가 차면 Partial 은 버린다
                ChunkEvent::Final { .. } => { let _ = tx.send(job); }
            }
        }
    }
    if let Some(ev) = chunker.flush() { let _ = tx.send(Job { ev, enqueued: Instant::now() }); }
}

fn transcribe_loop(rx: Receiver<Job>, t: &mut dyn Transcriber, lang: Option<&str>, tx: Sender<EngineEvent>) {
    let mut next_id = 1u64;
    let mut lagging = false;
    for job in rx {
        let waited = job.enqueued.elapsed();
        if waited > Duration::from_secs(10) && !lagging { lagging = true; let _ = tx.send(EngineEvent::Lagging { queued_ms: waited.as_millis() as u64 }); }
        if waited < Duration::from_secs(2) { lagging = false; }
        match job.ev {
            ChunkEvent::Partial { pcm, start_ms } => {
                if let Ok(segs) = t.transcribe(&pcm, lang) {
                    if let Some(s) = segs.into_iter().next() { let _ = tx.send(EngineEvent::Partial { text: s.text, lang: s.lang, start_ms }); }
                }
            }
            ChunkEvent::Final { pcm, start_ms, end_ms } => match t.transcribe(&pcm, lang) {
                Ok(segs) => {
                    if let Some(s) = segs.into_iter().next() {
                        let _ = tx.send(EngineEvent::Final { id: next_id, text: s.text, lang: s.lang, start_ms, end_ms });
                        next_id += 1;
                    }
                }
                Err(e) => { let _ = tx.send(EngineEvent::Error { code: "inference".into(), message: e.to_string() }); }
            },
        }
    }
    let _ = tx.send(EngineEvent::Stopped);
}
```

`Segment`의 `t0_ms/t1_ms`는 여기서 조각 오프셋으로 대체되므로 사용하지 않는다(필드는 3단계 타임스탬프용으로 유지). `TrySendError` import 가 미사용이면 제거한다.

- [ ] **Step 3: 게이트** — `mise exec -- cargo test -p babelay-engine --features metal engine`(1 PASS), clippy.

- [ ] **Step 4: Commit** — `feat(engine): capture→chunk→transcribe pipeline with events`

---

### Task 6: 하드웨어 감지와 balanced

**Files:**
- Create: `crates/babelay-engine/src/hardware.rs`
- Modify: `Cargo.toml`, `lib.rs`, `models.rs`(`BALANCED` 제거 → `balanced(&HwInfo)`), `src-tauri/src/models.rs`(`balanced` 계산 교체), `src-tauri/src/commands.rs`(`get_hw_info`), `src-tauri/src/lib.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Debug, serde::Serialize)] pub struct HwInfo { pub chip: String, pub mem_gb: u32, pub gpu: Option<String>, pub gpu_mem_gb: Option<u32> }
  pub fn detect() -> HwInfo;
  pub struct Balanced { pub asr: &'static str, pub llm: &'static str }
  pub fn balanced(hw: &HwInfo) -> Balanced;   // 표: GPU&&mem>=16 → large-v3-turbo/qwen3.5-4b; GPU&&mem>=8 → small/qwen3.5-2b; else base/gemma3-1b (mem = gpu_mem_gb.unwrap_or(mem_gb))
  ```
  커맨드 `get_hw_info() -> HwInfo`.

- [ ] **Step 1: 테스트**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn hw(gpu: bool, mem: u32, vram: Option<u32>) -> HwInfo { HwInfo { chip: "x".into(), mem_gb: mem, gpu: gpu.then(|| "g".to_string()), gpu_mem_gb: vram } }
    #[test] fn gpu_16gb_is_turbo_and_4b() { let b = balanced(&hw(true, 16, None)); assert_eq!((b.asr, b.llm), ("large-v3-turbo", "qwen3.5-4b")); }
    #[test] fn gpu_8gb_is_small_and_2b() { let b = balanced(&hw(true, 8, None)); assert_eq!((b.asr, b.llm), ("small", "qwen3.5-2b")); }
    #[test] fn cpu_only_is_base_and_gemma() { let b = balanced(&hw(false, 64, None)); assert_eq!((b.asr, b.llm), ("base", "gemma3-1b")); }
    #[test] fn nvidia_uses_vram_not_ram() { let b = balanced(&hw(true, 64, Some(6))); assert_eq!(b.asr, "base"); }
    #[test] fn balanced_ids_exist() { let b = balanced(&hw(true, 16, None)); assert!(crate::models::find(b.asr).is_some() && crate::models::find(b.llm).is_some()); }
}
```

- [ ] **Step 2: 구현**

```toml
sysinfo = "0.39"
[target.'cfg(target_os = "windows")'.dependencies]
nvml-wrapper = "0.13"
```

```rust
use sysinfo::System;

#[derive(Clone, Debug, serde::Serialize)]
pub struct HwInfo { pub chip: String, pub mem_gb: u32, pub gpu: Option<String>, pub gpu_mem_gb: Option<u32> }

pub struct Balanced { pub asr: &'static str, pub llm: &'static str }

pub fn detect() -> HwInfo {
    let sys = System::new_all();
    let chip = sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
    let mem_gb = (sys.total_memory() / (1 << 30)) as u32;
    let (gpu, gpu_mem_gb) = gpu_info();
    HwInfo { chip, mem_gb, gpu, gpu_mem_gb }
}

#[cfg(target_os = "macos")]
fn gpu_info() -> (Option<String>, Option<u32>) {
    if cfg!(target_arch = "aarch64") { (Some("Apple Silicon (Metal)".into()), None) } else { (None, None) }
}

#[cfg(target_os = "windows")]
fn gpu_info() -> (Option<String>, Option<u32>) {
    let Ok(nvml) = nvml_wrapper::Nvml::init() else { return (None, None) };
    let Ok(dev) = nvml.device_by_index(0) else { return (None, None) };
    let name = dev.name().ok();
    let vram = dev.memory_info().ok().map(|m| (m.total / (1 << 30)) as u32);
    (name, vram)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn gpu_info() -> (Option<String>, Option<u32>) { (None, None) }

// ponytail: 고정 표. 실측 후 조정.
pub fn balanced(hw: &HwInfo) -> Balanced {
    let mem = hw.gpu_mem_gb.unwrap_or(hw.mem_gb);
    match (hw.gpu.is_some(), mem) {
        (true, m) if m >= 16 => Balanced { asr: "large-v3-turbo", llm: "qwen3.5-4b" },
        (true, m) if m >= 8 => Balanced { asr: "small", llm: "qwen3.5-2b" },
        _ => Balanced { asr: "base", llm: "gemma3-1b" },
    }
}
```

`models.rs`의 `BALANCED` 상수와 `Balanced` 구조체를 제거하고, 해당 테스트(`balanced_ids_exist_with_matching_kind`)는 hardware.rs 테스트로 대체한다. `src-tauri/src/models.rs`의 `list()`는 `hardware::balanced(&hardware::detect())`를 한 번 계산해 쓴다(호출마다 `System::new_all()`은 수십 ms — `OnceLock<HwInfo>`로 캐시). `get_hw_info` 커맨드 추가·등록.

- [ ] **Step 3: 게이트** — 엔진 테스트, clippy, Windows check(`nvml-wrapper`는 링크 없이 check 가능).

- [ ] **Step 4: Commit** — `feat: hardware detection and spec-based balanced recommendation`

---

### Task 7: 세션 커맨드와 이벤트 중계 (src-tauri)

**Files:**
- Create: `src-tauri/src/session.rs`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/commands.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/lib.rs`, `src-tauri/tauri.windows.conf.json`(신규), `README.md`

**Interfaces:**
- Consumes: `babelay_engine::engine::{start_default, EngineConfig, EngineEvent, EngineHandle}`, `capture::probe_permission`, `models::{find, model_path}`, `SettingsState`
- Produces:
  - 커맨드 `start_capture() -> Result<(), String>`(모델 미설치면 `Err("model_missing")`, 이미 실행 중이면 no-op), `stop_capture()`, `capture_state() -> bool`, `check_audio_permission()`(실제 probe: `"granted"|"denied"|"unknown"`)
  - 이벤트 `engine-event`(EngineEvent JSON, 모든 창)
  - 트레이/단축키 `toggle_capture` → `session::toggle(app)`; 트레이 캡처 라벨 start/stop 갱신(`tray::relabel`에 `capturing` 반영)
  - `Final` 이벤트는 `history::insert_segment`(Task 8)로 적재 — 이 태스크에서는 훅 자리만(`on_final: fn(&AppHandle, &EngineEvent)`) 두고 Task 8에서 채운다.

- [ ] **Step 1: Cargo**

```toml
[target.'cfg(target_os = "macos")'.dependencies]
babelay-engine = { path = "../crates/babelay-engine", features = ["metal"] }
[target.'cfg(target_os = "windows")'.dependencies]
babelay-engine = { path = "../crates/babelay-engine", features = ["cuda"] }
```

기존 무조건 의존은 제거한다(feature 없는 Linux 빌드는 지원 밖).

- [ ] **Step 2: session.rs**

```rust
use crate::{models::models_dir, settings::SettingsState};
use babelay_engine::engine::{start_default, EngineConfig, EngineEvent, EngineHandle};
use babelay_engine::models::{find, model_path};
use std::sync::{mpsc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Default)]
pub struct SessionState { handle: Mutex<Option<EngineHandle>> }

fn lock(s: &SessionState) -> std::sync::MutexGuard<'_, Option<EngineHandle>> { s.handle.lock().unwrap_or_else(|p| p.into_inner()) }

pub fn is_capturing(app: &AppHandle) -> bool { lock(&app.state::<SessionState>()).is_some() }

pub fn start(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<SessionState>();
    if lock(&state).is_some() { return Ok(()); }
    let settings = app.state::<SettingsState>().get();
    let m = find(&settings.asr.model_id).ok_or("unknown model")?;
    let path = model_path(&models_dir(app)?, m);
    if !babelay_engine::models::installed(&models_dir(app)?, m) { return Err("model_missing".into()); }
    let cfg = EngineConfig { model_path: path, use_gpu: settings.asr.gpu, source_lang: (settings.asr.source_lang != "auto").then(|| settings.asr.source_lang.clone()) };
    let (tx, rx) = mpsc::channel();
    let handle = start_default(cfg, tx)?;
    *lock(&state) = Some(handle);
    let app2 = app.clone();
    std::thread::spawn(move || {
        for ev in rx {
            if let EngineEvent::Final { .. } = &ev { crate::history::on_final(&app2, &ev); }
            let stopped = matches!(ev, EngineEvent::Stopped);
            let _ = app2.emit("engine-event", &ev);
            if stopped { break; }
        }
        let _ = crate::tray::relabel_capture(&app2, false);
    });
    let _ = crate::tray::relabel_capture(app, true);
    Ok(())
}

pub fn stop(app: &AppHandle) {
    if let Some(h) = lock(&app.state::<SessionState>()).take() { h.stop(); }
}

pub fn toggle(app: &AppHandle) -> Result<(), String> {
    if is_capturing(app) { stop(app); Ok(()) } else { start(app) }
}
```

`history::on_final`은 Task 8 전까지 `pub fn on_final(_: &AppHandle, _: &EngineEvent) {}`로 둔다(`src-tauri/src/history.rs` 스텁 생성). `tray::relabel_capture(app, capturing)`은 `TrayItems.capture`의 텍스트를 `start`/`stop`으로 바꾼다(`i18n::TrayLabels.stop` 사용 → `#[allow(dead_code)]` 제거).

- [ ] **Step 3: 커맨드·트레이·lib**

`commands.rs`: `start_capture`(→ `session::start`, 오류는 그대로 문자열), `stop_capture`, `capture_state`, `check_audio_permission`을 `babelay_engine::capture::probe_permission()`으로 교체(Windows는 항상 granted), `get_hw_info`(Task 6). `tray.rs`의 `toggle_capture`는 `session::toggle(app)` 호출 후 오류를 `app.emit("engine-event", EngineEvent::Error{code:"start_failed", message})`로 알린다; 기존 `capture-toggle` 이벤트는 삭제. `lib.rs`: `mod session; mod history;`, `.manage(SessionState::default())`, 커맨드 등록. 앱 종료 시(`RunEvent::Exit`) `session::stop`.

- [ ] **Step 4: Windows CUDA 리소스(설정만)**

`src-tauri/tauri.windows.conf.json`:

```json
{ "bundle": { "resources": ["resources/cuda/*.dll"] } }
```

`src-tauri/resources/cuda/.gitkeep`을 추가하고 README 빌드 절에 "Windows: CUDA Toolkit 설치 후 `cudart64_12.dll`, `cublas64_12.dll`, `cublasLt64_12.dll`을 `src-tauri/resources/cuda/`에 복사하고 `yarn tauri build`" 한 줄을 넣는다. 이 머신에서는 검증 불가하므로 보고서에 그렇게 적는다.

- [ ] **Step 5: 게이트** — 6개 게이트 + `mise exec -- yarn tauri build --debug --no-bundle`. 실행 확인: `mise exec -- yarn tauri dev` → 트레이 "캡처 시작"을 누르면 (모델이 설치되어 있고 권한이 있으면) 콘솔에 `engine-event` 로그가 흐른다. 확인용으로 `main.tsx`에 임시 `listen("engine-event", console.log)`를 넣었다면 커밋 전에 지운다.

- [ ] **Step 6: Commit** — `feat: capture session commands, engine event relay, real permission probe`

---

### Task 8: SQLite 히스토리

**Files:**
- Create/Modify: `src-tauri/src/history.rs`(스텁 교체), `src-tauri/Cargo.toml`(rusqlite), `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/session.rs`(세션 시작/종료 시 `history::begin/end`)

**Interfaces:**
- Produces:
  ```rust
  pub struct Db(Mutex<rusqlite::Connection>);      // app.manage
  pub fn open(path: &Path) -> rusqlite::Result<Db>; // 스키마 생성(IF NOT EXISTS)
  pub fn begin(app, src_lang: &str, tgt_lang: &str, asr_model: &str) -> i64   // sessions 행, 현재 id 를 SessionState 에 보관
  pub fn end(app)                                    // ended_at 갱신
  pub fn on_final(app, &EngineEvent)                 // segments + segments_fts 삽입
  #[derive(Serialize)] pub struct SessionSummary { id, started_at, ended_at: Option<i64>, src_lang, tgt_lang, asr_model, segments: i64 }
  #[derive(Serialize)] pub struct SegmentRow { id, session_id, t0_ms, t1_ms, lang, src_text, tgt_text: Option<String> }
  ```
  커맨드: `history_sessions(limit: u32) -> Vec<SessionSummary>`(최신순), `history_segments(session_id) -> Vec<SegmentRow>`, `history_search(q) -> Vec<SegmentRow>`(FTS5 MATCH, 최대 200), `history_delete(session_id)`, `history_export(session_id, format: "txt"|"srt") -> String`(다운로드 폴더에 `babelay-<session_id>.<ext>` 저장 후 경로 반환).

- [ ] **Step 1: 테스트(인메모리)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn insert_search_and_export() {
        let db = open_in_memory().unwrap();
        let sid = db.begin_session("en", "ko", "small").unwrap();
        db.insert_segment(sid, 0, 1200, "en", "hello world").unwrap();
        db.insert_segment(sid, 1200, 2500, "en", "second line").unwrap();
        db.end_session(sid).unwrap();
        assert_eq!(db.sessions(10).unwrap()[0].segments, 2);
        assert_eq!(db.search("world").unwrap().len(), 1);
        let srt = db.export(sid, "srt").unwrap();
        assert!(srt.starts_with("1\n00:00:00,000 --> 00:00:01,200\nhello world\n\n2\n"));
        let txt = db.export(sid, "txt").unwrap();
        assert_eq!(txt, "hello world\nsecond line\n");
        db.delete_session(sid).unwrap();
        assert!(db.sessions(10).unwrap().is_empty());
        assert!(db.search("world").unwrap().is_empty());
    }
}
```

- [ ] **Step 2: 구현 개요(코드는 브리프 그대로 작성)**

```toml
rusqlite = { version = "0.40", features = ["bundled"] }
```

스키마:

```sql
CREATE TABLE IF NOT EXISTS sessions(id INTEGER PRIMARY KEY, started_at INTEGER NOT NULL, ended_at INTEGER, src_lang TEXT, tgt_lang TEXT, asr_model TEXT, translator TEXT);
CREATE TABLE IF NOT EXISTS segments(id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE, t0_ms INTEGER, t1_ms INTEGER, lang TEXT, src_text TEXT NOT NULL, tgt_text TEXT);
CREATE VIRTUAL TABLE IF NOT EXISTS segments_fts USING fts5(src_text, tgt_text, content='segments', content_rowid='id');
CREATE TRIGGER IF NOT EXISTS segments_ai AFTER INSERT ON segments BEGIN INSERT INTO segments_fts(rowid, src_text, tgt_text) VALUES (new.id, new.src_text, new.tgt_text); END;
CREATE TRIGGER IF NOT EXISTS segments_ad AFTER DELETE ON segments BEGIN INSERT INTO segments_fts(segments_fts, rowid, src_text, tgt_text) VALUES('delete', old.id, old.src_text, old.tgt_text); END;
PRAGMA foreign_keys = ON;
```

`Db` 메서드: `begin_session`, `end_session`, `insert_segment`, `sessions(limit)`(`LEFT JOIN` count), `segments(session_id)`, `search(q)`(`WHERE segments_fts MATCH ?1 ORDER BY rank LIMIT 200`, `q`는 따옴표로 감싸 phrase 검색: `format!("\"{}\"", q.replace('"', ""))`), `delete_session`, `export(session_id, fmt)`. SRT 시간 포맷 `HH:MM:SS,mmm`. DB 파일은 `app_local_data_dir/history.sqlite`. 커맨드 `history_export`는 `app.path().download_dir()`에 파일을 쓰고 경로를 돌려준다. `session::start`에서 `begin`, 이벤트 스레드의 `Stopped`에서 `end`. `on_final`은 현재 세션 id(`SessionState`에 `session_id: Mutex<Option<i64>>`)로 삽입.

- [ ] **Step 3: 게이트 + Commit** — `feat: sqlite session history with search and export`

---

### Task 9: 프론트엔드 — 세션 스토어, 라이브, 오버레이, 히스토리, 사양 한 줄

**Files:**
- Modify: `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/session.ts`, `src/main.tsx`, `src/components/Sidebar.tsx`, `src/pages/main/Live.tsx`, `src/pages/main/History.tsx`, `src/pages/OverlayWindow.tsx`, `src/pages/settings/Models.tsx`, `src/pages/Onboarding.tsx`, `src/locales/*.json`
- Create: `src/test/session.test.ts`

**Interfaces:**
- Produces:
  - `types.ts`: `EngineEvent`(태그드 유니온, Rust와 동일 키), `SessionSummary`, `SegmentRow`, `HwInfo`
  - `api`: `startCapture`, `stopCapture`, `captureState`, `getHwInfo`, `historySessions(limit)`, `historySegments(id)`, `historySearch(q)`, `historyDelete(id)`, `historyExport(id, format)`
  - `session.ts`: 순수 리듀서 `reduce(state, ev): SessionView` — `{ capturing, gpuFallback, lagging, partial: {text,lang,start_ms}|null, finals: Final[] (최대 500), lastEventAt }`; 스토어 `useSession { view, start(), stop(), bind() }`; `bind`는 `engine-event`를 구독해 `reduce`, `Error`는 `setError`. `start` 실패 `model_missing`은 로케일 키 `errors.modelMissing`.
- 규칙: 오버레이는 최신 `Final` 원문을 큰 줄에, `Partial`을 작은 줄(흐리게)에; 표시 모드 `target`이어도 3단계 전까지 원문을 보여준다(번역 없음). 마지막 이벤트 후 6초 지나면 페이드아웃(opacity 0, transition 0.5s). 라이브 타임라인은 `finals`를 시간 순으로, 하단에 `partial`. 상태 필: 캡처 중(초록 점), GPU 폴백(회색 배지 "CPU"), 지연(회색 배지 "지연"). 사이드바 점은 `view.capturing`. 히스토리: 세션 목록(날짜·길이·언어·조각 수) → 클릭 상세(조각 타임라인) → 검색 입력(300ms 디바운스, 결과는 세션 링크 포함) → TXT/SRT 내보내기 버튼(저장 경로를 ErrorBar가 아닌 짧은 토스트 텍스트로 2초 표시) → 삭제. 설정 > 모델 상단에 `HwInfo` 한 줄("Apple M2 · 16 GB · Apple Silicon (Metal)"). 온보딩 권한 단계는 실제 `check_audio_permission` 결과를 표시(denied면 "시스템 설정 열기" 강조).

- [ ] **Step 1: 리듀서 테스트**

```ts
import { describe, it, expect } from "vitest";
import { reduce, initialView } from "../lib/session";

describe("session reducer", () => {
  it("started/stopped toggle capturing and flags", () => {
    let v = reduce(initialView, { type: "started", gpu_active: false, gpu_fallback: true });
    expect(v.capturing).toBe(true); expect(v.gpuFallback).toBe(true);
    v = reduce(v, { type: "stopped" });
    expect(v.capturing).toBe(false); expect(v.partial).toBeNull();
  });
  it("partial is replaced by final and finals are capped", () => {
    let v = reduce(initialView, { type: "partial", text: "hel", lang: "en", start_ms: 0 });
    expect(v.partial?.text).toBe("hel");
    v = reduce(v, { type: "final", id: 1, text: "hello", lang: "en", start_ms: 0, end_ms: 900 });
    expect(v.partial).toBeNull(); expect(v.finals).toHaveLength(1);
    for (let i = 2; i <= 600; i++) v = reduce(v, { type: "final", id: i, text: "x", lang: "en", start_ms: i, end_ms: i + 1 });
    expect(v.finals).toHaveLength(500); expect(v.finals[0].id).toBe(101);
  });
  it("lagging sets and a later final clears it", () => {
    let v = reduce(initialView, { type: "lagging", queued_ms: 12000 });
    expect(v.lagging).toBe(true);
    v = reduce(v, { type: "final", id: 1, text: "a", lang: "en", start_ms: 0, end_ms: 1 });
    expect(v.lagging).toBe(false);
  });
});
```

- [ ] **Step 2: 구현** — 위 규칙대로. `OverlayWindow.tsx`의 샘플 문장 로직(`ponytail:`)을 `useSession` 뷰로 교체하고 페이드 타이머 추가. `Live.tsx`의 시작/정지는 `api.startCapture/stopCapture`(오류는 `setError`). 새 로케일 키: `live.cpuFallback`("CPU"/"CPU"/"CPU"), `live.lagging`("지연"/"Lagging"/"遅延"), `history.search`("검색"/"Search"/"検索"), `history.exportTxt`("TXT"), `history.exportSrt`("SRT"), `history.delete`("삭제"/"Delete"/"削除"), `history.saved`("저장됨: {{path}}"/"Saved: {{path}}"/"保存済み: {{path}}"), `history.segments`("{{count}}개 조각"/"{{count}} segments"/"{{count}} セグメント"), `errors.modelMissing`("선택한 전사 모델이 설치되어 있지 않습니다"/"The selected transcription model is not installed"/"選択した文字起こしモデルがインストールされていません"), `errors.startFailed`("캡처를 시작할 수 없습니다"/"Could not start capture"/"キャプチャを開始できません"). 삭제: `overlay.sampleSource`, `overlay.sampleTarget`(설정 > 오버레이 미리보기는 고정 예문을 로케일 키 `overlay.previewSource/previewTarget`으로 유지 — 즉 이름만 바꾼다).

- [ ] **Step 3: 게이트 + 수동 확인** — 6개 게이트, `yarn tauri dev`로: 라이브 시작 → 오디오 재생 → 타임라인과 오버레이에 원문이 흐름, 정지 → 히스토리에 세션이 생김, 검색·내보내기·삭제 동작.

- [ ] **Step 4: Commit** — `feat(ui): live transcript, overlay text, history page, hardware line`

---

### Task 10: 문서와 체크리스트

**Files:**
- Modify: `README.md`(테스트 절에 `--ignored` 캡처/모델 테스트 안내, Windows CUDA DLL), `docs/superpowers/specs/2026-09-02-babelay-design.md`(§4.1 ObjC 심, §3.1 리샘플 자체 구현, §11 2단계 완료 표시)
- Create: `docs/superpowers/2026-09-03-phase2-gui-checklist.md`

- [ ] **Step 1: 스펙 갱신**: §4.1 "바인딩은 `objc2-core-audio`…" → "Core Audio Process Tap은 `cc`로 컴파일하는 ObjC 심(`crates/babelay-engine/csrc/tap.m`)이 만들고 C ABI 3개(`babelay_tap_start/stop/probe`)로 노출한다"; §3.1 "리샘플: … (rubato)" → "(선형 보간 자체 구현)"; §11 2단계 줄 끝에 "— 완료(날짜)".
- [ ] **Step 2: 체크리스트 작성**(권한 프롬프트, 캡처 시작/정지, 부분/확정 자막, GPU 폴백 강제 방법 = 설정에서 GPU 끔, 지연 표시, 히스토리 검색/내보내기, 다중 모니터 오버레이).
- [ ] **Step 3: Commit** — `docs: phase 2 spec updates and GUI checklist`

---

## 완료 기준

- macOS에서 시스템 오디오 재생 중 라이브 시작 → 2초 내 Partial, 문장 끝 무음 후 Final이 타임라인과 오버레이에 표시.
- 트레이·단축키로 캡처 토글, 트레이 라벨이 시작/정지로 바뀜.
- GPU 토글 off/on 모두 동작, Metal 초기화 실패 시 "CPU" 배지.
- 세션 종료 후 히스토리에 세션·조각이 저장되고 검색·TXT/SRT 내보내기·삭제 동작.
- 설정 > 모델 상단에 사양 한 줄, balanced 배지가 사양 표를 따름(16GB Apple Silicon → Large v3 Turbo / Qwen 3.5 4B).
- Windows 코드는 `cargo check --target x86_64-pc-windows-msvc` 통과(런타임 검증은 Windows 머신에서).
- 여섯 게이트 전부 통과.
