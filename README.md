# Babelay

시스템 오디오를 실시간으로 전사·번역해 화면 위 자막으로 보여주는 데스크톱 앱 (macOS 14.2+, Windows 10+).

## 개발

    yarn install
    yarn tauri dev

## 테스트

    yarn test && cargo test --workspace

## 빌드

macOS 서명 빌드에는 아래 환경변수가 필요하다.

    APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
    APPLE_ID=... APPLE_PASSWORD=<앱 암호> APPLE_TEAM_ID=...   # 공증
    yarn tauri build

GitHub Actions 시크릿: `APPLE_CERTIFICATE`(p12 base64), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`.

## 문서

- 설계: `docs/superpowers/specs/2026-09-02-babelay-design.md`
- 목업: `docs/design/mockups/`
