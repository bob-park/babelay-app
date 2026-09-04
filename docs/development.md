# Babelay 개발 가이드

사용자용 소개와 설치는 [README](../README.md)에 있다. 이 문서는 소스에서 빌드하고 검증하는 방법이다.

## 사전 준비

- [mise](https://mise.jdx.dev) (`.mise.toml`이 node 24, yarn 4.18.0, cmake 4를 고정)
- Rust stable 툴체인
- cmake는 `.mise.toml`이 제공한다. 별도 설치 없이 `mise exec -- ...`로 명령을 돌리면 whisper.cpp 빌드에 필요한 cmake가 잡힌다.

## 개발

    yarn install
    yarn tauri dev

## 검증

머지 전에 아래 네 가지가 모두 통과해야 한다(로컬 게이트, CI 없음).

    yarn tsc --noEmit
    yarn test
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings

포매팅은 `cargo fmt --all`로 맞추고 `cargo fmt --all -- --check`로 확인한다.

### 무시된 테스트

시스템 오디오 권한이나 모델 파일이 필요한 두 테스트는 `#[ignore]`라 위 게이트에서 빠진다. 직접 돌릴 때만 쓴다.

시스템 오디오 캡처(macOS, 시스템 오디오 녹음 권한 필요 — 먼저 아무 소리나 재생해 둔다):

    mise exec -- cargo test -p babelay-engine captures_some_frames -- --ignored --nocapture

whisper 전사(`ggml-*.bin` 모델 파일 필요):

    BABELAY_TEST_MODEL=<ggml-*.bin 경로> mise exec -- cargo test -p babelay-engine --features metal transcribes_synthetic -- --ignored

엔드투엔드(실제 탭 + Whisper, GUI 없음): 음악 대신 `say`가 문장을 읽고 자막 이벤트가 출력된다.

    BABELAY_TEST_MODEL=<path to ggml-*.bin> mise exec -- cargo run -p babelay-engine --features metal --example e2e

로컬 LLM 번역(GGUF 모델 파일 필요, 첫 llama.cpp 빌드는 수 분):

    BABELAY_TEST_LLM="$HOME/Library/Application Support/org.bobpark.babelay/models/llm/Qwen3.5-2B-Q4_K_M.gguf" mise exec -- cargo test -p babelay-engine --features metal translates_english_to_korean -- --ignored --nocapture

키체인 라운드트립(macOS 는 접근 프롬프트가 뜰 수 있다):

    mise exec -- cargo test -p babelay roundtrip -- --ignored

### API 키

클라우드 번역의 API 키는 OS 자격 증명 저장소(macOS Keychain / Windows Credential Manager, 서비스 `org.bobpark.babelay`, 계정 = 프로바이더)에만 저장되고 `settings.json`에는 들어가지 않는다. 설정 › 번역 › 클라우드 API에서 키를 저장한 뒤 `연결 테스트`를 누르면 짧은 문장을 실제로 번역해 응답 시간과 결과를 보여준다(로컬 모델에서도 동작).

### macOS 개발 실행과 시스템 오디오 권한

`yarn tauri dev`로 띄운 앱은 그 터미널을 실행한 앱(Terminal, iTerm, RustRover 등) 기준으로 시스템 오디오 녹음 권한을 판단한다. 권한이 없으면 프롬프트 없이 무음이 들어와 자막이 나오지 않는다. 시스템 설정 → 개인정보 보호 및 보안 → 화면 및 시스템 오디오 녹음에서 해당 터미널/IDE를 허용한 뒤 다시 실행한다. 배포 빌드(.app)는 앱 자체에 권한을 묻는다.

## 빌드

macOS 서명 빌드에는 아래 환경변수가 필요하다.

    APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
    APPLE_ID=... APPLE_PASSWORD=<앱 암호> APPLE_TEAM_ID=...   # 공증
    yarn tauri build

빌드는 로컬에서만 한다(CI 없음). Windows 빌드는 Windows 머신에서 만들며 서명하지 않는다.

### Windows

`src-tauri/Cargo.toml`이 Windows에서 `cuda` 피처를 강제하므로 CUDA 툴체인이 필수다.

1. **Visual Studio 2022 Build Tools** — "C++를 사용한 데스크톱 개발" 워크로드. CUDA 12.8은 VS 2026(MSVC 14.5x)을 지원하지 않으므로 2022가 따로 있어야 한다.
2. **CUDA Toolkit 12.8** — VS 2022 설치 *후에* 설치하고 "Visual Studio Integration" 항목을 켠다. 설치기가 `CUDA_PATH`를 잡는다.
3. **LLVM** — `winget install LLVM.LLVM`(관리자). bindgen이 `libclang.dll`을 쓴다.
4. **CMake** — `winget install Kitware.CMake`. Windows에서는 mise 대신 시스템 cmake를 쓴다.
5. **환경변수**(사용자 변수, 설정 후 터미널/IDE 재시작):

       LIBCLANG_PATH=C:\Program Files\LLVM\bin
       CMAKE_GENERATOR=Visual Studio 17 2022
       VSLANG=1033                      # MSVC 메시지를 영어로(한글 깨짐 방지)

6. **CUDA 런타임 DLL** — `%CUDA_PATH%\bin`의 `cudart64_12.dll`, `cublas64_12.dll`, `cublasLt64_12.dll`을 `src-tauri/resources/cuda/`에 복사. `tauri.windows.conf.json`의 `"resources/cuda/*.dll": "./"` 매핑이 exe 옆에 놓는다.
7. **Windows Defender 제외** — 프로젝트 폴더, `%USERPROFILE%\.cargo`. 빼지 않으면 빌드 산출물 스캔으로 EBUSY가 나거나 매우 느리다.

빌드는 **"x64 Native Tools Command Prompt for VS 2022"** 에서 한다. 일반 터미널은 `INCLUDE`가 비어 있어 bindgen이 `stdbool.h`를 못 찾고, 그러면 동봉된 Linux 바인딩으로 대체돼 `12_usize - 16_usize` 오버플로 오류가 난다.

    mise exec -- yarn install
    mise exec -- yarn tauri dev      # 개발
    mise exec -- yarn tauri build    # 배포: src-tauri/target/release/bundle/

첫 빌드는 whisper.cpp·llama.cpp를 CUDA로 컴파일하므로 수십 분 걸린다. 개발 머신에서는 `CMAKE_CUDA_ARCHITECTURES=<내 GPU sm>`(예: RTX 40 = 89)으로 줄일 수 있다. 배포 빌드에서는 두지 않는다.

문제 해결: `cargo build 2> build.log` 뒤 `findstr /C:"Unable to generate bindings" build.log`가 잡히면 bindgen 실패이고, 그 이유 줄이 진짜 원인이다.

## 문서

- 설계: `superpowers/specs/2026-09-02-babelay-design.md`
- 2단계 GUI 수동 체크리스트: `superpowers/2026-09-03-phase2-gui-checklist.md`
- 3단계(번역) GUI 수동 체크리스트: `superpowers/2026-09-03-phase3-gui-checklist.md`
- 4단계(패스쓰루·장치 변경) GUI 수동 체크리스트: `superpowers/2026-09-04-phase4-gui-checklist.md`
- 목업: `design/mockups/`
