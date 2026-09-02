# Babelay

시스템 오디오를 실시간으로 전사·번역해 화면 위 자막으로 보여주는 데스크톱 앱 (macOS 14.2+, Windows 10+).

## 사전 준비

- [mise](https://mise.jdx.dev) (`.mise.toml`이 node 24, yarn 4.18.0을 고정)
- Rust stable 툴체인

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

## 빌드

macOS 서명 빌드에는 아래 환경변수가 필요하다.

    APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
    APPLE_ID=... APPLE_PASSWORD=<앱 암호> APPLE_TEAM_ID=...   # 공증
    yarn tauri build

빌드는 로컬에서만 한다(CI 없음). Windows 빌드는 Windows 머신에서 `yarn tauri build`로 만들며 서명하지 않는다.
Windows: CUDA Toolkit 설치 후 `cudart64_12.dll`, `cublas64_12.dll`, `cublasLt64_12.dll`을 `src-tauri/resources/cuda/`에 복사하고 `yarn tauri build` — 번들에서 exe 옆에 놓인다(`tauri.windows.conf.json`).

## 문서

- 설계: `docs/superpowers/specs/2026-09-02-babelay-design.md`
- 목업: `docs/design/mockups/`
