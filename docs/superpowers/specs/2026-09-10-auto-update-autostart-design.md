# 자동 업데이트 · 로그인 시 자동 실행 설계

2026-09-10. 브랜치 `feature/auto-update`.

## 목표

- GitHub Releases 를 기준으로 새 버전을 찾고, 앱 안에서 내려받아 설치·재시작한다.
- 설정에서 자동 확인을 켜고 끈다. 켜져 있으면 주기적으로 확인해 "업데이트 가능" 을 표시한다. 설치는 사용자가 누를 때만.
- 수동 확인·설치 버튼을 둔다.
- 로그인 시 자동 실행을 설정에서 켜고 끈다. 자동 실행 시에도 메인 창을 띄운다(직접 실행과 동일).

## 결정

| 갈림길 | 선택 | 이유 |
|---|---|---|
| 설치 방식 | `tauri-plugin-updater` 로 앱 내 설치 | 사용자가 dmg/exe 를 다시 열지 않게 |
| 확인 주체 | Rust 비동기 태스크 | 창을 닫아도 트레이에서 살아 있으므로 |
| `latest.json` 생성 | 로컬 스크립트 `scripts/latest-json.mjs` | 빌드는 로컬(CI 없음). mac·Windows 를 다른 기계에서 빌드하므로 릴리스에 올라온 `.sig` 를 모아 조립 |
| 자동 실행 상태 저장 | OS 등록 상태가 진실 | `settings.json` 과 어긋날 일을 없앤다 |
| 자동 확인 시 다운로드 | 하지 않음 | 요구사항은 "표시". 설치는 사용자 클릭 |

## 1. 업데이트 백엔드 (Rust)

의존성: `tauri-plugin-updater = "2"`. 재시작은 `AppHandle::restart` 로 충분하므로 process 플러그인은 붙이지 않는다.

`src-tauri/src/updater.rs`

- `UpdateState(Mutex<Option<Update>>)`: 마지막 확인에서 발견한 업데이트 하나.
- `UpdateInfo { version: String, notes: String }` (serde, 프론트로 나가는 형태).
- `async fn check(app) -> Result<Option<UpdateInfo>, String>`: `app.updater()?.check().await`. 있으면 상태에 저장하고 `update-available` 이벤트(payload `UpdateInfo`) 발행, 없으면 상태를 비운다. 끝에 `tray::relabel_update(app)`.
- `fn spawn_periodic(app)`: setup 에서 한 번 호출. `tauri::async_runtime::spawn` 안에서 10초 대기 후 무한 루프: `settings.general.auto_update` 가 true 면 `check`, 그리고 24시간 대기. 설정을 매 주기마다 다시 읽으므로 토글이 즉시 반영된다. 오류는 `eprintln!` 로만 남긴다(오프라인이 정상 상황).
- `async fn install(app) -> Result<(), String>`: 상태에서 `Update` 를 꺼낸다(없으면 오류 `no_update`). `update.download(progress, done)` 로 받으며 `update-progress { received, total }` 를 발행. 다운로드가 끝나면 **먼저** 종료 정리(`session::stop_on_exit`, `llm::cache(app).clear()`)를 하고 `update.install(bytes)` → `app.restart()`. Windows 는 `install` 이 설치기를 띄우고 프로세스를 끝내므로 정리가 그 앞에 있어야 한다. 종료 정리는 `lib.rs` 의 `RunEvent::Exit` 핸들러와 같은 함수를 공유한다(`fn shutdown(app)` 으로 빼서 두 곳에서 호출).

커맨드 (`commands.rs`, `lib.rs` 핸들러 등록)

- `update_status() -> Option<UpdateInfo>`: 보관 중인 정보. 창이 나중에 열렸을 때 초기 표시용.
- `check_update() -> Option<UpdateInfo>`: 수동 확인.
- `install_update() -> ()`: 설치 시작. 진행은 이벤트로.

설정

- `General.auto_update: bool`, 기본 `true`. `defaults_match_spec`, `mutated_roundtrip` 테스트 갱신.

트레이 (`tray.rs`, `i18n.rs`)

- "Babelay 열기" 아래 항목 `update` 하나 추가. 라벨은 업데이트가 없으면 "업데이트 확인", 있으면 "v{version} 설치". 클릭: 보관된 업데이트가 있으면 `install`, 없으면 `check`. 항목 하나가 수동 확인과 설치를 겸한다.
- `TrayLabels` 에 `check_update`, `install_update`(`{}` 자리에 버전) 추가. `relabel` 은 언어가 바뀔 때 이 항목도 다시 쓴다. `relabel_update(app)` 는 `UpdateState` 를 읽어 라벨만 바꾼다.

`tauri.conf.json`

```json
"bundle": { "createUpdaterArtifacts": true },
"plugins": {
  "updater": {
    "pubkey": "<minisign 공개키 본문>",
    "endpoints": ["https://github.com/bob-park/babelay-app/releases/latest/download/latest.json"],
    "windows": { "installMode": "passive" }
  }
}
```

Rust 에서만 호출하므로 capability 는 추가하지 않는다.

## 2. 프론트

- `types.ts` `Settings.general.auto_update: boolean`, `settings.ts` 기본값 `true`. `UpdateInfo`, `UpdateProgress` 타입.
- `tauri.ts` `api`: `updateStatus`, `checkUpdate`, `installUpdate`, `getAutostart`, `setAutostart`.
- `src/lib/update.ts`: zustand 스토어 `{ info: UpdateInfo | null, progress: UpdateProgress | null, checking, error }`. `subscribe()` 가 `update-available` / `update-progress` 를 듣고 마운트 시 `updateStatus` 로 초기화. `check()`, `install()`.
- `pages/settings/General.tsx` 에 "업데이트" `SettingGroup`:
  - 행 1: "자동으로 확인" 토글 ↔ `general.auto_update`.
  - 행 2: 현재 버전(`getVersion()` from `@tauri-apps/api/app`) + 상태 문구("최신 버전" / "v0.3.0 사용 가능" / "확인 중") + 버튼("지금 확인" 또는 "설치"). 설치 중이면 버튼 대신 `progress`.
  - 행 3: "로그인 시 자동 실행" 토글. 마운트 시 `getAutostart` 로 초기값, 변경 시 `setAutostart`. 실패하면 `showError` 토스트로 표시.
- `Sidebar.tsx`: 설정 항목에 업데이트 대기 시 점 하나.
- 로케일 `ko/en/ja.json` 에 `update.*`, `general.autostart` 키.

### 알림 (추가 요구, Task 10)
모든 알림은 react-toastify 로 보여준다. 오류는 `src/lib/toast.ts` 의 `showError` (같은 메시지는 `toastId` 로 하나로 합침), 다운로드 진행·업데이트 발견(설치 버튼)·설치 진행은 `src/components/Toasts.tsx` 가 스토어를 관찰해 띄운다. 스토어는 토스트를 모른다. 외관은 daisyUI 색 변수를 toastify CSS 변수에 매핑해 기존 UI 를 따른다. 히스토리 내보내기 완료와 번역 연결 테스트 결과도 토스트다.

## 3. 로그인 시 자동 실행

- `tauri-plugin-autostart = "2"`. setup 에서 `tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None)`.
- 커맨드 `get_autostart() -> bool` (`autolaunch().is_enabled()`), `set_autostart(enabled: bool)` (`enable()`/`disable()`).
- capability 불필요(Rust 호출).

## 4. 릴리스 절차

빌드는 지금처럼 로컬. 달라지는 점만.

1. 최초 1회: `yarn tauri signer generate -w ~/.tauri/babelay.key`. 공개키를 `tauri.conf.json` 에 넣는다. 개인키와 비밀번호는 `~/.config/babelay/sign.env` 에 `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 로 추가한다. Windows 빌드 기계에도 같은 두 변수가 필요하다. **개인키를 잃으면 기존 설치본에 업데이트를 보낼 수 없다** — 백업한다.
2. 버전 올리고(`package.json`, `Cargo.toml` 두 개, `tauri.conf.json`) `v<버전>` 태그.
3. 각 기계에서 `yarn tauri build`. 산출물:
   - macOS `target/release/bundle/`: `dmg/Babelay_<v>_aarch64.dmg`(공증·스테이플 후), `macos/Babelay.app.tar.gz`, `macos/Babelay.app.tar.gz.sig`
   - Windows: `nsis/Babelay_<v>_x64-setup.exe`, `nsis/Babelay_<v>_x64-setup.exe.sig`
4. 릴리스에 위 파일을 모두 올린다(`gh release create` 또는 `gh release upload`). 순서 무관.
5. 어느 기계에서든 `node scripts/latest-json.mjs v<버전>`:
   - `gh release download <tag> -p '*.sig'` 로 서명을 받는다.
   - `*.app.tar.gz.sig` → `darwin-aarch64`, `*-setup.exe.sig` → `windows-x86_64`. URL 은 `https://github.com/bob-park/babelay-app/releases/download/<tag>/<sig 파일명에서 .sig 를 뗀 것>`.
   - 존재하는 플랫폼만 넣는다(업데이터는 나열된 블록이 모두 완전해야 파일 전체를 받아들인다).
   - `{ version, pub_date(now, RFC 3339), platforms }` 를 `latest.json` 으로 쓰고 `gh release upload <tag> latest.json --clobber`.
   - 순수 함수 `buildManifest(version, sigs: {name, body}[])` 를 export 해 테스트한다.
6. 릴리스를 게시하면 그 순간부터 `releases/latest/download/latest.json` 이 이 버전을 가리킨다.

`docs/development.md` 빌드 절에 1·3·5 를 적는다. 0.2.0 사용자는 업데이터가 없으므로 다음 버전을 한 번 직접 설치한다.

## 오류 처리

- 확인 실패(오프라인, 404, 서명 불일치): 주기 확인은 조용히 로그만. 수동 확인은 커맨드 오류 → `showError` 토스트(react-toastify).
- 설치 실패: `update-progress` 대신 커맨드 오류. 상태의 `Update` 는 유지해 다시 시도할 수 있다.
- `latest.json` 에 현재 플랫폼 항목이 없으면 플러그인이 "없음" 으로 돌려준다 → "최신 버전" 표시. 이 경우 실제로는 아직 그 플랫폼 빌드가 안 올라온 것이지만 구분하지 않는다.

## 테스트

- Rust: 설정 기본값·라운드트립에 `auto_update`; `tray_labels_are_localized` 에 새 라벨; `updater` 의 `UpdateInfo` 직렬화.
- vitest: `scripts/latest-json.test.ts` — `buildManifest` 가 플랫폼 키·URL·서명 본문을 맞게 만들고, 한쪽 `.sig` 만 있으면 그 플랫폼만 넣는지. 설정 스토어 기본값 테스트 갱신.
- 수동: dev 앱에서 자동 실행 토글 → `~/Library/LaunchAgents/org.bobpark.babelay.plist` 생성·삭제 확인. 업데이트 끝단은 첫 서명 릴리스(0.3.0) 게시 후 0.3.0 설치본에서 0.3.1 확인·설치·재시작으로 검증.

## 범위 밖

- CI 빌드. 자동 다운로드. 릴리스 노트 렌더링(`notes` 는 한 줄 텍스트로만). 델타 업데이트.
