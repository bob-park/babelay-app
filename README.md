# Babelay

시스템 오디오를 실시간으로 전사·번역해 화면 위 자막으로 보여주는 데스크톱 앱 (macOS 14.2+, Windows 10+).

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

### macOS 개발 실행과 시스템 오디오 권한

`yarn tauri dev`로 띄운 앱은 그 터미널을 실행한 앱(Terminal, iTerm, RustRover 등) 기준으로 시스템 오디오 녹음 권한을 판단한다. 권한이 없으면 프롬프트 없이 무음이 들어와 자막이 나오지 않는다. 시스템 설정 → 개인정보 보호 및 보안 → 화면 및 시스템 오디오 녹음에서 해당 터미널/IDE를 허용한 뒤 다시 실행한다. 배포 빌드(.app)는 앱 자체에 권한을 묻는다.

## 빌드

macOS 서명 빌드에는 아래 환경변수가 필요하다.

    APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
    APPLE_ID=... APPLE_PASSWORD=<앱 암호> APPLE_TEAM_ID=...   # 공증
    yarn tauri build

빌드는 로컬에서만 한다(CI 없음). Windows 빌드는 Windows 머신에서 `yarn tauri build`로 만들며 서명하지 않는다.
Windows: CUDA Toolkit 설치 후 `cudart64_12.dll`, `cublas64_12.dll`, `cublasLt64_12.dll`을 `src-tauri/resources/cuda/`에 복사하고 `yarn tauri build` — `tauri.windows.conf.json`의 `"resources/cuda/*.dll": "./"` 매핑이 번들에서 exe 옆에 놓는다.

## 문서

- 설계: `docs/superpowers/specs/2026-09-02-babelay-design.md`
- 2단계 GUI 수동 체크리스트: `docs/superpowers/2026-09-03-phase2-gui-checklist.md`
- 목업: `docs/design/mockups/`
