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

시스템 오디오 권한이나 모델 파일이 필요한 테스트들은 `#[ignore]`라 위 게이트에서 빠진다. 직접 돌릴 때만 쓴다.

시스템 오디오 캡처(macOS, 시스템 오디오 녹음 권한 필요 — 먼저 아무 소리나 재생해 둔다):

    mise exec -- cargo test -p babelay-engine captures_some_frames -- --ignored --nocapture

whisper 전사(`ggml-*.bin` 모델 파일 필요):

    BABELAY_TEST_MODEL=<ggml-*.bin 경로> mise exec -- cargo test -p babelay-engine --features metal transcribes_synthetic -- --ignored

Qwen3-ASR 전사(llama.cpp mtmd, 본체 GGUF 와 mmproj 두 파일 필요. `BABELAY_TEST_ASR_WAV` 로 16 kHz 모노 16-bit WAV 를 주면 무음 대신 그 파일을 전사한다):

    BABELAY_TEST_ASR_GGUF=<Qwen3-ASR-*.gguf> BABELAY_TEST_ASR_MMPROJ=<mmproj-*.gguf> mise exec -- cargo test -p babelay-engine --features metal loads_and_transcribes_silence -- --ignored --nocapture

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

값은 `~/.config/babelay/sign.env`(`KEY=VALUE` env 형식, export 없이)에 두고 `set -a; source ~/.config/babelay/sign.env; set +a` 로 읽는다.
이 변수 없이 빌드하면 번들이 서명되지 않아 내려받은 사용자에게 "손상된 앱" 으로 뜬다(Apple Silicon Gatekeeper).
Tauri 는 `.app` 만 공증·스테이플하고 `.dmg` 는 서명만 하므로, 릴리스에 올리기 전에 dmg 도 한 번 더 공증한다.

    xcrun notarytool submit target/release/bundle/dmg/Babelay_<버전>_aarch64.dmg \
      --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
    xcrun stapler staple target/release/bundle/dmg/Babelay_<버전>_aarch64.dmg
    spctl --assess --type open --context context:primary-signature -v target/release/bundle/dmg/Babelay_<버전>_aarch64.dmg   # accepted 여야 한다

빌드는 로컬에서만 한다(CI 없음). Windows 빌드는 Windows 머신에서 만들며 Apple 서명은 없다.

### 업데이터 서명 키 (최초 1회)

앱 내 자동 업데이트는 minisign 서명을 검증한다. 서명은 끌 수 없다.

    yarn tauri signer generate -w ~/.tauri/babelay.key

공개키(`~/.tauri/babelay.key.pub` 내용)를 `src-tauri/tauri.conf.json` 의 `plugins.updater.pubkey` 에 넣는다. 개인키와 비밀번호는 `~/.config/babelay/sign.env` 에 추가한다. Windows 빌드 기계에도 같은 두 변수가 필요하다.

    TAURI_SIGNING_PRIVATE_KEY=<개인키 파일 경로 또는 내용>
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD=<비밀번호>

**개인키를 잃으면 이미 설치된 앱에 업데이트를 보낼 수 없다.** 백업한다. 변수 없이 빌드하면 `.sig` 가 만들어지지 않아 업데이트로 배포할 수 없다.

### 릴리스 절차

1. 버전을 올린다: `package.json`, `Cargo.toml`(루트·`src-tauri`·`crates/babelay-engine`), `src-tauri/tauri.conf.json`. `v<버전>` 태그를 민다.
2. 각 기계에서 `yarn tauri build`. `bundle.createUpdaterArtifacts` 가 켜져 있어 아래가 나온다.
   - macOS `src-tauri/target/release/bundle/`: `dmg/Babelay_<버전>_aarch64.dmg`(위 공증·스테이플 후), `macos/Babelay.app.tar.gz`, `macos/Babelay.app.tar.gz.sig`
   - Windows: `nsis/Babelay_<버전>_x64-setup.exe`, `nsis/Babelay_<버전>_x64-setup.exe.sig`
3. 릴리스를 만들고 위 파일을 모두 올린다. 순서는 상관없다.

       gh release create v<버전> --generate-notes <파일들>     # 처음
       gh release upload v<버전> <파일들>                       # 다른 기계에서 추가

4. 아무 기계에서나 한 번:

       node scripts/latest-json.mjs v<버전>

   릴리스의 `.sig` 를 내려받아 `latest.json` 을 조립해 올린다. 있는 플랫폼만 들어가므로 한쪽만 올라온 상태에서 돌려도 되고, 나머지를 올린 뒤 다시 돌리면 덮어쓴다. 공개키(plugins.updater.pubkey)가 비어 있으면 스크립트가 업로드 전에 멈춘다.
5. 릴리스를 게시하면 `releases/latest/download/latest.json` 이 이 버전을 가리키고, 설치된 앱이 다음 확인(시작 10초 후, 이후 24시간마다)에서 알린다.

0.2.0 이전 설치본에는 업데이터가 없으므로 그 사용자는 한 번 직접 설치해야 한다.

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
7. **ggml 중복 심볼** — whisper-rs-sys 와 llama-cpp-sys-2 가 각자 ggml 을 묶어 오므로 `.cargo/config.toml` 이 MSVC 에 `/FORCE:MULTIPLE` 을 준다. 링크 중 `LNK4006`(multiply defined) 경고가 수백 줄 나오는 건 정상이고, 이 설정을 지우면 `LNK2005` → `LNK1169` 로 실패한다.
8. **Windows Defender 제외** — 프로젝트 폴더, `%USERPROFILE%\.cargo`. 빼지 않으면 빌드 산출물 스캔으로 EBUSY가 나거나 매우 느리다.

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
