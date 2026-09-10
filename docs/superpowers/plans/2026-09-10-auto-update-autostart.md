# 자동 업데이트 · 로그인 자동 실행 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** GitHub Releases 기준으로 새 버전을 찾아 앱 안에서 설치·재시작하고, 설정에서 자동 확인과 로그인 자동 실행을 켜고 끈다.

**Architecture:** Rust 가 `tauri-plugin-updater` 로 확인·설치를 맡고(창이 닫혀도 트레이에서 살아 있으므로), 결과를 이벤트로 프론트와 트레이 라벨에 알린다. 자동 실행은 `tauri-plugin-autostart` 이고 OS 등록 상태가 진실이다. `latest.json` 은 로컬 스크립트가 릴리스의 `.sig` 를 모아 조립한다.

**Tech Stack:** Tauri 2 (Rust 1.88), tauri-plugin-updater 2, tauri-plugin-autostart 2, tokio(time), React 19 + zustand + react-i18next, vitest, `gh` CLI.

**Spec:** `docs/superpowers/specs/2026-09-10-auto-update-autostart-design.md`

## Global Constraints

- 브랜치 `feature/auto-update` 에서 작업. 커밋 메시지는 기존 관례(`feat(scope): 한국어 요약`).
- 설정 필드는 스네이크 케이스, Rust `Settings` ↔ TS `Settings` 계약을 양쪽 기본값 테스트로 지킨다.
- 로케일 ko/en/ja 세 파일은 키 집합이 같아야 한다(`src/test/locales.test.ts` 가 강제).
- 머지 게이트: `yarn tsc --noEmit`, `yarn test`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. Rust 명령은 `mise exec -- cargo ...` 로(cmake 잡힘). 엔진 빌드가 처음이면 수 분 걸린다.
- 자동 확인 시 다운로드 없음. 설치는 사용자 클릭. 주기: 시작 10초 후 + 24시간.
- Windows 는 `install()` 이 설치기를 띄우고 프로세스를 끝내므로 종료 정리(`shutdown`)가 `install` **앞에** 와야 한다.
- 업데이터 개인키는 저장소에 넣지 않는다. 공개키만 `tauri.conf.json`.
- ponytail: 추상화·파일 추가 최소. 새 파일은 `updater.rs`, `update.ts`, `latest-json.mjs`, 테스트 두 개뿐.

---

## 파일 구조

| 파일 | 역할 |
|---|---|
| `src-tauri/src/settings.rs` | `General.auto_update` 추가 |
| `src-tauri/src/updater.rs` (신규) | `UpdateState`, `UpdateInfo`, `check`, `install`, `spawn_periodic`, `on_tray_click` |
| `src-tauri/src/lib.rs` | 플러그인 등록, `shutdown` 헬퍼, 주기 확인 시작, 커맨드 등록 |
| `src-tauri/src/commands.rs` | `update_status`, `check_update`, `install_update`, `get_autostart`, `set_autostart` |
| `src-tauri/src/i18n.rs` | 트레이 라벨 2개 |
| `src-tauri/src/tray.rs` | 트레이 항목 `update`, `relabel_update` |
| `src-tauri/Cargo.toml`, `tauri.conf.json` | 의존성, updater 설정 |
| `src/lib/types.ts`, `settings.ts`, `tauri.ts` | 타입·기본값·API |
| `src/lib/update.ts` (신규) | 업데이트 스토어 |
| `src/pages/settings/General.tsx`, `src/components/Sidebar.tsx`, `src/pages/MainApp.tsx` | UI |
| `src/locales/{ko,en,ja}.json` | 문구 |
| `scripts/latest-json.mjs` (신규), `src/test/latest-json.test.ts` (신규), `tsconfig.json` | 매니페스트 스크립트 |
| `docs/development.md` | 릴리스 절차 |

---

### Task 1: 설정 `auto_update`

**Files:**
- Modify: `src-tauri/src/settings.rs` (struct `General`, `impl Default for General`, tests `mutated_roundtrip`, `defaults_match_spec`)
- Modify: `src/lib/types.ts` (interface `Settings.general`)
- Modify: `src/lib/settings.ts` (`defaultSettings.general`)
- Test: `src/test/settings.test.ts`

**Interfaces:**
- Produces: `settings.general.auto_update: bool` (Rust) / `boolean` (TS), 기본 `true`. Task 2·6 이 읽는다.

- [ ] **Step 1: Rust 테스트 먼저 수정**

`src-tauri/src/settings.rs` 의 `mutated_roundtrip` 에서 `s.general.theme = "dark".into();` 다음 줄에 추가:

```rust
        s.general.auto_update = false;
```

`defaults_match_spec` 에서 `assert!(!s.general.onboarding_done);` 다음 줄에 추가:

```rust
        assert!(s.general.auto_update);
```

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && mise exec -- cargo test settings::tests`
Expected: 컴파일 오류 `no field auto_update on type General`

- [ ] **Step 3: 필드 추가**

`struct General`:

```rust
pub struct General {
    pub theme: String,       // system | dark | light
    pub ui_language: String, // system | ko | en | ja
    pub onboarding_done: bool,
    pub auto_update: bool, // 주기적으로 새 버전을 확인할지. 설치는 언제나 사용자 클릭.
}
```

`impl Default for General`:

```rust
        Self {
            theme: "system".into(),
            ui_language: "system".into(),
            onboarding_done: false,
            auto_update: true,
        }
```

- [ ] **Step 4: 통과 확인**

Run: `cd src-tauri && mise exec -- cargo test settings::tests`
Expected: 6 passed

- [ ] **Step 5: TS 테스트 추가**

`src/test/settings.test.ts` 의 `describe("mergeSettings", ...)` 안에 케이스 추가:

```ts
  it("defaults auto_update to on", () => {
    expect(defaultSettings.general.auto_update).toBe(true);
  });
```

Run: `yarn test src/test/settings.test.ts`
Expected: FAIL (`undefined` is not `true`)

- [ ] **Step 6: TS 타입·기본값**

`src/lib/types.ts`:

```ts
  general: { theme: Theme; ui_language: UiLang; onboarding_done: boolean; auto_update: boolean };
```

`src/lib/settings.ts`:

```ts
  general: { theme: "system", ui_language: "system", onboarding_done: false, auto_update: true },
```

- [ ] **Step 7: 통과 확인**

Run: `yarn test src/test/settings.test.ts && yarn tsc --noEmit`
Expected: PASS, tsc 오류 없음

- [ ] **Step 8: 커밋**

```bash
git add src-tauri/src/settings.rs src/lib/types.ts src/lib/settings.ts src/test/settings.test.ts
git commit -m "feat(settings): general.auto_update 필드(기본 켜짐)"
```

---

### Task 2: 업데이터 모듈과 커맨드

**Files:**
- Modify: `src-tauri/Cargo.toml` (`[dependencies]`)
- Modify: `src-tauri/tauri.conf.json` (`bundle`, 최상위 `plugins`)
- Create: `src-tauri/src/updater.rs`
- Modify: `src-tauri/src/lib.rs` (mod 선언, 플러그인, setup, 핸들러, run 클로저)
- Modify: `src-tauri/src/commands.rs` (끝에 3개 커맨드)

**Interfaces:**
- Consumes: `SettingsState::get().general.auto_update` (Task 1), `session::stop_on_exit(&AppHandle)`, `llm::cache(&AppHandle).clear()`.
- Produces:
  - `crate::shutdown(app: &AppHandle)` — 종료·재시작 전 정리.
  - `updater::UpdateInfo { version: String, notes: String }` (Serialize).
  - `updater::status(app: &AppHandle) -> Option<UpdateInfo>`
  - `updater::check(app: &AppHandle) -> Result<Option<UpdateInfo>, String>` (async)
  - `updater::install(app: &AppHandle) -> Result<(), String>` (async)
  - `updater::on_tray_click(app: AppHandle)` — Task 3 가 부른다.
  - 이벤트 `update-available` (UpdateInfo), `update-progress` (`{received: u64, total: Option<u64>}`), `update-error` (String).
  - 커맨드 `update_status`, `check_update`, `install_update`.
  - `tray::relabel_update(app)` 를 호출한다 — Task 3 에서 정의. **이 Task 에서는 tray.rs 에 빈 함수를 먼저 둔다**(아래 Step 5).

- [ ] **Step 1: 의존성**

`src-tauri/Cargo.toml` `[dependencies]` 에 추가:

```toml
tauri-plugin-updater = "2"
tokio = { version = "1", features = ["time"] } # 주기 확인 sleep
```

Run: `cd src-tauri && mise exec -- cargo fetch`
Expected: 오류 없음

- [ ] **Step 2: 실패하는 테스트가 담긴 모듈 파일 생성**

`src-tauri/src/updater.rs` 전체:

```rust
//! GitHub Releases 의 latest.json 을 보고 새 버전을 찾아 설치한다.
//! 창이 닫혀도 앱은 트레이에서 살아 있으므로 확인은 여기서(Rust) 돈다.

use crate::settings::SettingsState;
use serde::Serialize;
use std::{sync::Mutex, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

/// 마지막 확인에서 찾은 업데이트. 없으면 None.
#[derive(Default)]
pub struct UpdateState(Mutex<Option<Update>>);

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
}

impl UpdateInfo {
    fn from_update(u: &Update) -> Self {
        Self {
            version: u.version.clone(),
            notes: u.body.clone().unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct UpdateProgress {
    pub received: u64,
    pub total: Option<u64>,
}

const FIRST_CHECK: Duration = Duration::from_secs(10);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// 보관 중인 업데이트 정보. 상태가 아직 등록되지 않았으면(setup 초반) None.
pub fn status(app: &AppHandle) -> Option<UpdateInfo> {
    app.try_state::<UpdateState>()?
        .0
        .lock()
        .unwrap()
        .as_ref()
        .map(UpdateInfo::from_update)
}

pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let found = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let info = found.as_ref().map(UpdateInfo::from_update);
    *app.state::<UpdateState>().0.lock().unwrap() = found;
    if let Some(i) = &info {
        let _ = app.emit("update-available", i);
    }
    crate::tray::relabel_update(app);
    Ok(info)
}

/// 받고 → 엔진 정리 → 설치 → 재시작. Windows 는 install 이 설치기를 띄우고 프로세스를 끝내므로
/// 정리가 그 앞에 있어야 한다. 실패해도 보관한 Update 는 남겨 다시 시도할 수 있다.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .state::<UpdateState>()
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no_update".to_string())?;
    let mut received: u64 = 0;
    let mut last_pct = u64::MAX;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                // 청크마다 보내면 수천 번이다. 퍼센트가 바뀔 때만.
                let pct = total.map_or(0, |t| received * 100 / t.max(1));
                if pct != last_pct {
                    last_pct = pct;
                    let _ = app.emit("update-progress", UpdateProgress { received, total });
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    crate::shutdown(app);
    update.install(bytes).map_err(|e| e.to_string())?;
    app.restart()
}

/// setup 에서 한 번. 설정을 매 주기마다 다시 읽으므로 토글이 바로 반영된다.
/// 오프라인은 정상 상황이라 실패는 로그만 남긴다.
pub fn spawn_periodic(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if app.state::<SettingsState>().get().general.auto_update {
                if let Err(e) = check(&app).await {
                    eprintln!("babelay update: check failed: {e}");
                }
            }
            tokio::time::sleep(INTERVAL).await;
        }
    });
}

/// 트레이 항목 하나가 "확인" 과 "설치" 를 겸한다. 오류는 이벤트로 메인 창에 보낸다.
pub fn on_tray_click(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let r = if status(&app).is_some() {
            install(&app).await
        } else {
            check(&app).await.map(|_| ())
        };
        if let Err(e) = r {
            eprintln!("babelay update: {e}");
            let _ = app.emit("update-error", e);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_serializes_with_frontend_keys() {
        let v = serde_json::to_value(UpdateInfo {
            version: "0.3.0".into(),
            notes: "fix".into(),
        })
        .unwrap();
        assert_eq!(v, serde_json::json!({ "version": "0.3.0", "notes": "fix" }));
        let p = serde_json::to_value(UpdateProgress {
            received: 5,
            total: None,
        })
        .unwrap();
        assert_eq!(p, serde_json::json!({ "received": 5, "total": null }));
    }
}
```

- [ ] **Step 3: lib.rs 배선**

`src-tauri/src/lib.rs`:

`mod tray;` 아래에 `mod updater;`.

`use tauri::Manager;` 아래에:

```rust
use tauri::AppHandle;
```

`.plugin(tauri_plugin_global_shortcut::Builder::new().build())` 다음 줄에:

```rust
        .plugin(tauri_plugin_updater::Builder::new().build())
```

setup 안 `app.manage(SessionState::default());` 다음 줄에:

```rust
            app.manage(updater::UpdateState::default());
```

setup 안 `tray::build(&handle)?;` 다음 줄에:

```rust
            updater::spawn_periodic(handle.clone());
```

`generate_handler!` 의 `commands::test_translation,` 다음에:

```rust
            commands::update_status,
            commands::check_update,
            commands::install_update,
```

`.run(...)` 클로저를 아래로 교체하고, 파일 끝에 `shutdown` 을 둔다:

```rust
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                shutdown(app);
            }
        });
}

/// 종료·재시작 전에 엔진을 세운다. 오디오 탭이 살아 있는 채로 프로세스가 죽으면 안 되고,
/// 번역 모델이 Metal 버퍼를 쥔 채 exit() 가 돌면 ggml 정적 소멸자가 abort 한다.
pub fn shutdown(app: &AppHandle) {
    session::stop_on_exit(app);
    llm::cache(app).clear();
}
```

- [ ] **Step 4: 커맨드**

`src-tauri/src/commands.rs` 끝에:

```rust
#[tauri::command]
pub fn update_status(app: AppHandle) -> Option<crate::updater::UpdateInfo> {
    crate::updater::status(&app)
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<crate::updater::UpdateInfo>, String> {
    crate::updater::check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    crate::updater::install(&app).await
}
```

- [ ] **Step 5: tray.rs 에 임시 빈 함수**

`src-tauri/src/tray.rs` 의 `relabel_capture` 함수 아래에(Task 3 에서 본문을 채운다):

```rust
/// 업데이트 항목 라벨 갱신. Task 3 에서 채운다.
pub fn relabel_update(_app: &AppHandle) {}
```

- [ ] **Step 6: tauri.conf.json**

`"bundle": { "active": true,` 를 `"bundle": { "active": true, "createUpdaterArtifacts": true,` 로. 그리고 `"bundle": { ... }` 블록 뒤(최상위)에 추가:

```json
  "plugins": {
    "updater": {
      "pubkey": "",
      "endpoints": ["https://github.com/bob-park/babelay-app/releases/latest/download/latest.json"],
      "windows": { "installMode": "passive" }
    }
  }
```

`pubkey` 채우기: `~/.tauri/babelay.key.pub` 파일이 있으면 그 내용(한 줄) 을 넣는다. 없으면 `""` 로 두고 최종 보고에 "`yarn tauri signer generate -w ~/.tauri/babelay.key` 로 키를 만들고 `.pub` 내용을 `plugins.updater.pubkey` 에 넣어야 한다" 고 적는다. 빈 공개키는 dev 에서 문제없고(서명 검증은 업데이트를 받을 때만) 릴리스 전에는 반드시 채워야 한다.

- [ ] **Step 7: 테스트·클리피**

Run: `cd src-tauri && mise exec -- cargo test updater::tests && mise exec -- cargo clippy --all-targets -- -D warnings`
Expected: 1 passed, clippy 경고 없음. (`app.restart()` 는 `!` 를 돌려주므로 `install` 의 마지막 식으로 그대로 둔다. clippy 가 `unreachable` 을 지적하면 `restart()` 뒤에 아무 것도 없는지 확인.)

- [ ] **Step 8: 커밋**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json src-tauri/src/updater.rs src-tauri/src/lib.rs src-tauri/src/commands.rs src-tauri/src/tray.rs
git commit -m "feat(update): tauri-plugin-updater 로 확인·설치, 24시간 주기 자동 확인"
```

---

### Task 3: 트레이 항목과 라벨

**Files:**
- Modify: `src-tauri/src/i18n.rs` (`TrayLabels`, `tray_labels`, test)
- Modify: `src-tauri/src/tray.rs` (`TrayItems`, `relabel`, `relabel_update`, `build`)

**Interfaces:**
- Consumes: `updater::status`, `updater::on_tray_click`, `updater::UpdateInfo` (Task 2).
- Produces: `tray::relabel_update(app: &AppHandle)` (Task 2 의 빈 함수를 채움), `i18n::TrayLabels { check_update, install_update }`.

- [ ] **Step 1: i18n 테스트 확장**

`src-tauri/src/i18n.rs` 의 `tray_labels_are_localized` 에 추가:

```rust
        assert_eq!(tray_labels(Lang::Ko).check_update, "업데이트 확인");
        assert_eq!(tray_labels(Lang::En).install_update, "Install v{}");
        assert_eq!(tray_labels(Lang::Ja).install_update, "v{} をインストール");
```

Run: `cd src-tauri && mise exec -- cargo test i18n::tests`
Expected: 컴파일 오류 (필드 없음)

- [ ] **Step 2: 라벨 추가**

`TrayLabels` 에 필드 두 개:

```rust
    pub quit: &'static str,
    pub check_update: &'static str,
    /// `{}` 자리에 버전
    pub install_update: &'static str,
```

각 언어 블록 `quit` 아래에:

```rust
            // Ko
            check_update: "업데이트 확인",
            install_update: "v{} 설치",
            // En
            check_update: "Check for Updates",
            install_update: "Install v{}",
            // Ja
            check_update: "アップデートを確認",
            install_update: "v{} をインストール",
```

Run: `cd src-tauri && mise exec -- cargo test i18n::tests`
Expected: 4 passed

- [ ] **Step 3: tray 라벨 함수 테스트**

`src-tauri/src/tray.rs` 파일 끝에:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{i18n::Lang, updater::UpdateInfo};

    #[test]
    fn update_label_follows_pending_state() {
        let l = i18n::tray_labels(Lang::Ko);
        assert_eq!(update_label(&l, None), "업데이트 확인");
        let info = UpdateInfo {
            version: "0.3.0".into(),
            notes: String::new(),
        };
        assert_eq!(update_label(&l, Some(info)), "v0.3.0 설치");
    }
}
```

Run: `cd src-tauri && mise exec -- cargo test tray::tests`
Expected: 컴파일 오류 (`update_label` 없음)

- [ ] **Step 4: tray.rs 구현**

`TrayItems` 에 필드:

```rust
pub struct TrayItems {
    pub capture: MenuItem<Wry>,
    pub overlay: MenuItem<Wry>,
    pub open: MenuItem<Wry>,
    pub update: MenuItem<Wry>,
    pub quit: MenuItem<Wry>,
}
```

`relabel` 의 `let _ = items.open.set_text(l.open);` 다음 줄에:

```rust
    let _ = items
        .update
        .set_text(update_label(&l, crate::updater::status(app)));
```

Task 2 의 빈 `relabel_update` 를 아래로 교체하고 `update_label` 을 `capture_label` 옆에 둔다:

```rust
/// 업데이트 확인이 끝날 때마다 호출된다. 트레이가 아직 없으면 무시된다.
pub fn relabel_update(app: &AppHandle) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let lang = i18n::resolve(&app.state::<SettingsState>().get().general.ui_language);
    let _ = items.update.set_text(update_label(
        &i18n::tray_labels(lang),
        crate::updater::status(app),
    ));
}

fn update_label(l: &i18n::TrayLabels, pending: Option<crate::updater::UpdateInfo>) -> String {
    match pending {
        Some(info) => l.install_update.replace("{}", &info.version),
        None => l.check_update.to_string(),
    }
}
```

`build` 에서 `let open = ...` 다음 줄에:

```rust
    let update_item = MenuItem::with_id(
        app,
        "update",
        update_label(&labels, crate::updater::status(app)),
        true,
        None::<&str>,
    )?;
```

메뉴 조립을 `.items(&[&open, &update_item])` 로. `on_menu_event` 에 분기:

```rust
            "update" => crate::updater::on_tray_click(app.clone()),
```

`app.manage(TrayItems { ... })` 에 `update: update_item,` 추가.

- [ ] **Step 5: 테스트·클리피**

Run: `cd src-tauri && mise exec -- cargo test tray::tests i18n::tests && mise exec -- cargo clippy --all-targets -- -D warnings`
Expected: 모두 통과

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src/i18n.rs src-tauri/src/tray.rs
git commit -m "feat(tray): 업데이트 확인/설치 항목(3개 언어)"
```

---

### Task 4: 로그인 시 자동 실행

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs` (플러그인, 핸들러)
- Modify: `src-tauri/src/commands.rs`

**Interfaces:**
- Produces: 커맨드 `get_autostart() -> bool`, `set_autostart(enabled: bool)`. Task 5 의 `api.getAutostart/setAutostart` 가 호출.

- [ ] **Step 1: 의존성**

`src-tauri/Cargo.toml` `[dependencies]` 에:

```toml
tauri-plugin-autostart = "2"
```

- [ ] **Step 2: 플러그인·커맨드**

`src-tauri/src/lib.rs` `.plugin(tauri_plugin_updater::Builder::new().build())` 다음 줄에:

```rust
        // 인자 없음: 자동 실행 때도 직접 실행과 같이 메인 창을 띄운다.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
```

`generate_handler!` 의 `commands::install_update,` 다음에:

```rust
            commands::get_autostart,
            commands::set_autostart,
```

`src-tauri/src/commands.rs` 끝에:

```rust
// 자동 실행 여부의 진실은 OS 등록(LaunchAgent / 레지스트리)이다. settings.json 에는 두지 않는다.
#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let m = app.autolaunch();
    if enabled { m.enable() } else { m.disable() }.map_err(|e| e.to_string())
}
```

- [ ] **Step 3: 빌드·클리피**

Run: `cd src-tauri && mise exec -- cargo clippy --all-targets -- -D warnings && mise exec -- cargo test`
Expected: 통과

- [ ] **Step 4: 커밋**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/src/commands.rs
git commit -m "feat(autostart): 로그인 시 자동 실행 커맨드(tauri-plugin-autostart)"
```

---

### Task 5: 프론트 타입·API·업데이트 스토어

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Create: `src/lib/update.ts`
- Create: `src/test/update.test.ts`

**Interfaces:**
- Consumes: 커맨드 이름 `update_status`, `check_update`, `install_update`, `get_autostart`, `set_autostart`; 이벤트 `update-available`, `update-progress`, `update-error` (Task 2·4); `useSettings.getState().setError`.
- Produces: `useUpdate` 스토어 `{ info, progress, checking, checked, check(), install(), subscribe() }`, `api.updateStatus/checkUpdate/installUpdate/getAutostart/setAutostart`, 타입 `UpdateInfo`, `UpdateProgress`.

- [ ] **Step 1: 실패하는 테스트**

`src/test/update.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { useUpdate } from "../lib/update";
import { useSettings } from "../lib/settings";
import { api } from "../lib/tauri";
import type { UpdateInfo, UpdateProgress } from "../lib/types";

vi.mock("../lib/tauri", () => ({
  api: { updateStatus: vi.fn(), checkUpdate: vi.fn(), installUpdate: vi.fn() },
}));

const h = vi.hoisted(() => ({ listeners: {} as Record<string, (e: { payload: unknown }) => void> }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, cb: (e: { payload: unknown }) => void) => {
    h.listeners[name] = cb;
    return Promise.resolve(() => {});
  },
}));

const info: UpdateInfo = { version: "0.3.0", notes: "" };

beforeEach(() => {
  useUpdate.setState({ info: null, progress: null, checking: false, checked: false });
  useSettings.setState({ error: null });
  vi.mocked(api.updateStatus).mockResolvedValue(null);
});

describe("useUpdate", () => {
  it("picks up a backend-found update via event", async () => {
    useUpdate.getState().subscribe();
    await Promise.resolve();
    h.listeners["update-available"]({ payload: info });
    expect(useUpdate.getState().info).toEqual(info);
  });

  it("check stores the result and marks checked", async () => {
    vi.mocked(api.checkUpdate).mockResolvedValueOnce(null);
    await useUpdate.getState().check();
    expect(useUpdate.getState()).toMatchObject({ info: null, checked: true, checking: false });
  });

  it("check failure lands in the settings error bar", async () => {
    vi.mocked(api.checkUpdate).mockRejectedValueOnce(new Error("offline"));
    await useUpdate.getState().check();
    expect(useSettings.getState().error).toBe("offline");
    expect(useUpdate.getState().checking).toBe(false);
  });

  it("install shows progress from events and clears it on failure", async () => {
    useUpdate.getState().subscribe();
    await Promise.resolve();
    vi.mocked(api.installUpdate).mockRejectedValueOnce(new Error("sig"));
    const p = useUpdate.getState().install();
    const prog: UpdateProgress = { received: 10, total: 100 };
    h.listeners["update-progress"]({ payload: prog });
    expect(useUpdate.getState().progress).toEqual(prog);
    await p;
    expect(useUpdate.getState().progress).toBeNull();
    expect(useSettings.getState().error).toBe("sig");
  });

  it("tray-triggered errors also surface", async () => {
    useUpdate.getState().subscribe();
    await Promise.resolve();
    h.listeners["update-error"]({ payload: "no_update" });
    expect(useSettings.getState().error).toBe("no_update");
  });
});
```

Run: `yarn test src/test/update.test.ts`
Expected: FAIL (`../lib/update` 없음)

- [ ] **Step 2: 타입·API**

`src/lib/types.ts` 의 `HwInfo` 줄 아래에:

```ts
export interface UpdateInfo { version: string; notes: string }
export interface UpdateProgress { received: number; total: number | null }
```

`src/lib/tauri.ts` import 에 `UpdateInfo` 추가하고 `api` 끝에:

```ts
  updateStatus: () => invoke<UpdateInfo | null>("update_status"),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
```

- [ ] **Step 3: 스토어**

`src/lib/update.ts`:

```ts
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { useSettings } from "./settings";
import type { UpdateInfo, UpdateProgress } from "./types";

// 확인은 Rust 가 주기적으로 돈다. 여기는 결과 표시와 버튼뿐.
interface UpdateStore {
  info: UpdateInfo | null;
  progress: UpdateProgress | null;
  checking: boolean;
  checked: boolean; // 수동 확인을 끝낸 뒤에만 "최신 버전" 을 말한다
  check: () => Promise<void>;
  install: () => Promise<void>;
  subscribe: () => () => void;
}

// 업데이트 오류도 설정 오류 배너 하나로 보여준다. 배너를 하나 더 만들 이유가 없다.
const fail = (e: unknown) => useSettings.getState().setError(e);

export const useUpdate = create<UpdateStore>((set) => ({
  info: null,
  progress: null,
  checking: false,
  checked: false,
  check: async () => {
    set({ checking: true });
    try {
      set({ info: await api.checkUpdate(), checked: true });
    } catch (e) {
      fail(e);
    } finally {
      set({ checking: false });
    }
  },
  install: async () => {
    set({ progress: { received: 0, total: null } });
    try {
      await api.installUpdate();
    } catch (e) {
      set({ progress: null });
      fail(e);
    }
  },
  subscribe: () => {
    // 창이 이벤트보다 늦게 열렸을 수 있다.
    api.updateStatus().then((info) => set({ info })).catch(() => {});
    const subs = [
      listen<UpdateInfo>("update-available", (e) => set({ info: e.payload })),
      listen<UpdateProgress>("update-progress", (e) => set({ progress: e.payload })),
      listen<string>("update-error", (e) => { set({ progress: null }); fail(e.payload); }),
    ];
    return () => { for (const p of subs) p.then((un) => un()); };
  },
}));
```

- [ ] **Step 4: 통과 확인**

Run: `yarn test src/test/update.test.ts && yarn tsc --noEmit`
Expected: 5 passed, tsc 오류 없음

- [ ] **Step 5: 커밋**

```bash
git add src/lib/types.ts src/lib/tauri.ts src/lib/update.ts src/test/update.test.ts
git commit -m "feat(update): 프론트 업데이트 스토어와 API 바인딩"
```

---

### Task 6: 일반 탭 UI · 사이드바 표시 · 로케일

**Files:**
- Modify: `src/locales/ko.json`, `src/locales/en.json`, `src/locales/ja.json` (`general` 블록, 새 `update` 블록)
- Modify: `src/pages/settings/General.tsx`
- Modify: `src/components/Sidebar.tsx`
- Modify: `src/pages/MainApp.tsx`
- Test: `src/test/locales.test.ts` (기존, 키 집합 일치를 강제)

**Interfaces:**
- Consumes: `useUpdate` (Task 5), `settings.general.auto_update` (Task 1), `api.getAutostart/setAutostart` (Task 5), `useSettings.setError`.

- [ ] **Step 1: 로케일**

세 파일의 `"general": { ... "permission": ... }` 블록 끝에 키 두 개를 추가하고, `"permission": {` 블록 **앞에** `update` 블록을 넣는다.

ko:

```json
    "permission": "권한", "autostart": "로그인 시 자동 실행", "updates": "업데이트"
  },
  "update": {
    "auto": "자동으로 확인", "version": "현재 버전", "check": "지금 확인", "checking": "확인 중…",
    "latest": "최신 버전", "available": "v{{version}} 사용 가능", "install": "설치", "installing": "설치 중…", "pending": "업데이트 대기 중"
  },
```

en:

```json
    "permission": "Permission", "autostart": "Launch at login", "updates": "Updates"
  },
  "update": {
    "auto": "Check automatically", "version": "Current version", "check": "Check now", "checking": "Checking…",
    "latest": "Up to date", "available": "v{{version}} available", "install": "Install", "installing": "Installing…", "pending": "Update pending"
  },
```

ja:

```json
    "permission": "権限", "autostart": "ログイン時に自動起動", "updates": "アップデート"
  },
  "update": {
    "auto": "自動的に確認", "version": "現在のバージョン", "check": "今すぐ確認", "checking": "確認中…",
    "latest": "最新の状態", "available": "v{{version}} が利用可能", "install": "インストール", "installing": "インストール中…", "pending": "アップデート待ち"
  },
```

Run: `yarn test src/test/locales.test.ts`
Expected: PASS (세 파일 키 집합 동일)

- [ ] **Step 2: General.tsx**

import 에 추가:

```ts
import { useUpdate } from "../../lib/update";
```

컴포넌트 안 `const [platform, ...]` 아래에:

```tsx
  const setError = useSettings((s) => s.setError);
  const { info, progress, checking, checked, check, install } = useUpdate();
  const [autostart, setAutostart] = useState<boolean | null>(null);
  useEffect(() => { api.getAutostart().then(setAutostart).catch(() => {}); }, []);
  const toggleAutostart = (on: boolean) => {
    setAutostart(on);
    api.setAutostart(on).catch((e) => { setAutostart(!on); setError(e); });
  };
```

`{platform === "macos" && (...)}` 블록 **앞에** 그룹 추가:

```tsx
      <div className="text-xs font-semibold uppercase tracking-wider text-fg-muted">{t("general.updates")}</div>
      <SettingGroup>
        <SettingRow label={t("update.auto")}>
          <input type="checkbox" role="switch" className="toggle toggle-primary" checked={settings.general.auto_update} onChange={(e) => update({ general: { auto_update: e.target.checked } })} />
        </SettingRow>
        <SettingRow label={`${t("update.version")} ${import.meta.env.PACKAGE_VERSION}`} as="div">
          {progress ? (
            <>
              <span className="text-xs">{t("update.installing")}</span>
              {progress.total ? <progress className="progress progress-primary h-1 w-24" value={progress.received} max={progress.total} /> : <progress className="progress progress-primary h-1 w-24" />}
            </>
          ) : info ? (
            <>
              <span className="text-xs">{t("update.available", { version: info.version })}</span>
              <button type="button" className="btn btn-primary btn-sm" onClick={install}>{t("update.install")}</button>
            </>
          ) : (
            <>
              <span className="text-xs">{checking ? t("update.checking") : checked ? t("update.latest") : ""}</span>
              <button type="button" className="btn btn-sm" disabled={checking} onClick={check}>{t("update.check")}</button>
            </>
          )}
        </SettingRow>
        <SettingRow label={t("general.autostart")}>
          <input type="checkbox" role="switch" className="toggle toggle-primary" checked={autostart ?? false} disabled={autostart === null} onChange={(e) => toggleAutostart(e.target.checked)} />
        </SettingRow>
      </SettingGroup>
```

- [ ] **Step 3: 스토어 구독(메인 창만)과 사이드바 점**

`src/pages/MainApp.tsx`: `import { useState } from "react";` → `import { useEffect, useState } from "react";`, import 에 `import { useUpdate } from "../lib/update";`. 컴포넌트 안 `const toggle = ...` 아래에:

```tsx
  useEffect(() => useUpdate.getState().subscribe(), []);
```

`src/components/Sidebar.tsx`: import 에 `import { useUpdate } from "../lib/update";`. `const asrInstalled = ...` 아래에:

```tsx
  const pending = useUpdate((s) => s.info !== null);
```

설정 `NavLink` 를 아래로 교체(창이 열려 있으면 트레이가 안 보이므로 여기서도 알린다):

```tsx
        <NavLink to="/settings/general" className={navCls} aria-label={t("nav.settings")}>
          <span className="indicator">
            {pending && <span className="badge badge-primary badge-xs indicator-item" aria-label={t("update.pending")} />}
            <Icon name="general" />
          </span>
          <span className={label}>{t("nav.settings")}</span>
        </NavLink>
```

- [ ] **Step 4: 게이트**

Run: `yarn tsc --noEmit && yarn test`
Expected: 통과

- [ ] **Step 5: 눈으로 확인**

Run: `mise exec -- yarn tauri dev`
확인: 설정 › 일반에 "업데이트" 그룹(토글, 버전+지금 확인, 자동 실행 토글). "지금 확인" 은 `pubkey` 가 비어 있거나 릴리스에 `latest.json` 이 없으면 오류 배너가 뜬다 — 이 단계에서는 정상. 자동 실행 토글을 켜면 `ls ~/Library/LaunchAgents/ | grep babelay` 에 plist 가 생기고, 끄면 사라진다. 트레이 메뉴에 "업데이트 확인" 항목이 "Babelay 열기" 아래에 있다.

- [ ] **Step 6: 커밋**

```bash
git add src/locales src/pages/settings/General.tsx src/components/Sidebar.tsx src/pages/MainApp.tsx
git commit -m "feat(ui): 일반 탭에 업데이트·자동 실행 설정, 사이드바 업데이트 표시"
```

---

### Task 7: `latest.json` 스크립트

**Files:**
- Create: `scripts/latest-json.mjs`
- Create: `src/test/latest-json.test.ts`
- Modify: `tsconfig.json` (`allowJs`)

**Interfaces:**
- Produces: `buildManifest(tag: string, sigs: {name: string, body: string}[], now?: Date)` → `{ version, pub_date, platforms }`. CLI: `node scripts/latest-json.mjs v0.3.0`.

- [ ] **Step 1: 실패하는 테스트**

`src/test/latest-json.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { buildManifest } from "../../scripts/latest-json.mjs";

const now = new Date("2026-09-10T00:00:00Z");
const mac = { name: "Babelay.app.tar.gz.sig", body: "MACSIG\n" };
const win = { name: "Babelay_0.3.0_x64-setup.exe.sig", body: "WINSIG" };

describe("buildManifest", () => {
  it("maps sig files to updater platform entries", () => {
    const m = buildManifest("v0.3.0", [mac, win], now);
    expect(m.version).toBe("0.3.0");
    expect(m.pub_date).toBe("2026-09-10T00:00:00.000Z");
    expect(m.platforms).toEqual({
      "darwin-aarch64": { signature: "MACSIG", url: "https://github.com/bob-park/babelay-app/releases/download/v0.3.0/Babelay.app.tar.gz" },
      "windows-x86_64": { signature: "WINSIG", url: "https://github.com/bob-park/babelay-app/releases/download/v0.3.0/Babelay_0.3.0_x64-setup.exe" },
    });
  });

  it("includes only platforms whose signature exists", () => {
    const m = buildManifest("v0.3.0", [mac], now);
    expect(Object.keys(m.platforms)).toEqual(["darwin-aarch64"]);
  });

  it("ignores unrelated files", () => {
    const m = buildManifest("v0.3.0", [{ name: "latest.json", body: "{}" }], now);
    expect(m.platforms).toEqual({});
  });
});
```

Run: `yarn test src/test/latest-json.test.ts`
Expected: FAIL (모듈 없음)

- [ ] **Step 2: 스크립트**

`scripts/latest-json.mjs`:

```js
#!/usr/bin/env node
// 릴리스에 올라온 .sig 자산을 모아 latest.json 을 만들고 같은 릴리스에 올린다.
// mac 과 Windows 를 다른 기계에서 빌드하므로, 둘 다 올린 뒤 아무 기계에서나 한 번 돌린다.
// 사용: node scripts/latest-json.mjs v0.3.0
import { execFileSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = "bob-park/babelay-app";
// 업데이터는 나열된 플랫폼 블록이 모두 완전해야 파일 전체를 받아들인다 — 있는 것만 넣는다.
const PLATFORMS = [
  { suffix: ".app.tar.gz.sig", key: "darwin-aarch64" },
  { suffix: "-setup.exe.sig", key: "windows-x86_64" },
];

/**
 * @param {string} tag  예: "v0.3.0"
 * @param {{name: string, body: string}[]} sigs  .sig 파일 이름과 내용
 * @param {Date} [now]
 */
export function buildManifest(tag, sigs, now = new Date()) {
  /** @type {Record<string, {signature: string, url: string}>} */
  const platforms = {};
  for (const { suffix, key } of PLATFORMS) {
    const sig = sigs.find((s) => s.name.endsWith(suffix));
    if (!sig) continue;
    const asset = sig.name.slice(0, -".sig".length);
    platforms[key] = {
      signature: sig.body.trim(),
      url: `https://github.com/${REPO}/releases/download/${tag}/${asset}`,
    };
  }
  return { version: tag.replace(/^v/, ""), pub_date: now.toISOString(), platforms };
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1]);
if (isMain) {
  const tag = process.argv[2];
  if (!tag) {
    console.error("usage: node scripts/latest-json.mjs <tag>");
    process.exit(1);
  }
  const dir = mkdtempSync(join(tmpdir(), "babelay-sig-"));
  execFileSync("gh", ["release", "download", tag, "-R", REPO, "-p", "*.sig", "-D", dir], { stdio: "inherit" });
  const sigs = readdirSync(dir).map((name) => ({ name, body: readFileSync(join(dir, name), "utf8") }));
  const manifest = buildManifest(tag, sigs);
  const found = Object.keys(manifest.platforms);
  if (found.length === 0) {
    console.error(`no .sig assets on release ${tag}`);
    process.exit(1);
  }
  const out = join(dir, "latest.json");
  writeFileSync(out, JSON.stringify(manifest, null, 2) + "\n");
  execFileSync("gh", ["release", "upload", tag, "-R", REPO, out, "--clobber"], { stdio: "inherit" });
  console.log(`latest.json uploaded to ${tag}: ${found.join(", ")}`);
}
```

- [ ] **Step 3: tsc 가 .mjs import 를 따라가게**

`tsconfig.json` `compilerOptions` 에 `"allowJs": true,` 추가(`"skipLibCheck": true,` 다음 줄). `include` 는 `src` 그대로 — 테스트가 import 하는 파일만 따라간다.

- [ ] **Step 4: 통과 확인**

Run: `yarn test src/test/latest-json.test.ts && yarn tsc --noEmit`
Expected: 3 passed, tsc 오류 없음

- [ ] **Step 5: CLI 인자 검사 확인**

Run: `node scripts/latest-json.mjs; echo "exit=$?"`
Expected: `usage: ...` 출력, `exit=1`

- [ ] **Step 6: 커밋**

```bash
git add scripts/latest-json.mjs src/test/latest-json.test.ts tsconfig.json
git commit -m "build(release): 릴리스의 .sig 를 모아 latest.json 을 올리는 스크립트"
```

---

### Task 8: 릴리스 문서

**Files:**
- Modify: `docs/development.md` (`## 빌드` 절)

- [ ] **Step 1: 문서 교체**

`## 빌드` 절에서 `빌드는 로컬에서만 한다(CI 없음). Windows 빌드는 Windows 머신에서 만들며 서명하지 않는다.` 줄을 아래로 교체:

```markdown
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

   릴리스의 `.sig` 를 내려받아 `latest.json` 을 조립해 올린다. 있는 플랫폼만 들어가므로 한쪽만 올라온 상태에서 돌려도 되고, 나머지를 올린 뒤 다시 돌리면 덮어쓴다.
5. 릴리스를 게시하면 `releases/latest/download/latest.json` 이 이 버전을 가리키고, 설치된 앱이 다음 확인(시작 10초 후, 이후 24시간마다)에서 알린다.

0.2.0 이전 설치본에는 업데이터가 없으므로 그 사용자는 한 번 직접 설치해야 한다.
```

- [ ] **Step 2: 커밋**

```bash
git add docs/development.md
git commit -m "docs: 업데이터 서명 키와 릴리스 절차"
```

---

### Task 9: 최종 게이트

**Files:** 없음(검증만)

- [ ] **Step 1: 전체 게이트**

Run:

```bash
yarn tsc --noEmit && yarn test && mise exec -- cargo fmt --all -- --check && mise exec -- cargo test --workspace && mise exec -- cargo clippy --workspace --all-targets -- -D warnings
```

Expected: 모두 통과. `fmt --check` 가 실패하면 `mise exec -- cargo fmt --all` 후 `style: fmt` 로 커밋.

- [ ] **Step 2: 보고**

최종 보고에 반드시 적는다:
- `plugins.updater.pubkey` 가 채워졌는지, 비었으면 키 생성 절차(Task 2 Step 6).
- 업데이트 끝단(확인 → 설치 → 재시작)은 첫 서명 릴리스를 게시한 뒤 이전 버전 설치본에서만 검증할 수 있다는 점.
- Windows 경로(`install()` 이 프로세스를 끝냄)는 Windows 기계에서 확인이 필요하다는 점.

---

## 자체 검토 결과

- 스펙 §1: Task 1·2·3. §2: Task 5·6. §3: Task 4·6. §4: Task 7·8. 오류 처리(주기 확인은 로그만, 수동은 배너, 설치 실패 시 Update 유지): Task 2 `spawn_periodic`/`install`, Task 5 `fail`. 테스트 절: Task 1·2·3·5·7 에 각각 있음.
- 이름 일치: `relabel_update`(Task 2 호출, Task 3 정의), `on_tray_click`(Task 2 정의, Task 3 호출), `UpdateInfo{version, notes}` ↔ TS `UpdateInfo`, 이벤트 이름 세 개, 커맨드 다섯 개 ↔ `api` 키.
- `shutdown` 은 `lib.rs` 의 `pub fn` 이고 `updater.rs` 는 `crate::shutdown` 으로 부른다.

---

### Task 10: 알림을 react-toastify 로 통일 (추가 요구사항, Task 6 뒤·Task 7 앞에 실행)

**요구:** 오류 배너(`ErrorBar`), 모델 다운로드 진행(`DownloadToast`), 히스토리 내보내기 완료 안내, 번역 연결 테스트 결과, 업데이트 발견 알림(설치 버튼), 업데이트 설치 진행률 — 전부 react-toastify 토스트로. 스토어는 토스트를 모른다(순수 유지, 테스트 그대로). 토스트를 띄우는 곳은 두 파일뿐: 오류는 `src/lib/toast.ts` 의 `showError`, 나머지는 `src/components/Toasts.tsx` 가 스토어를 관찰해 띄운다.

**Files:**
- Modify: `package.json` (`react-toastify` 의존성)
- Create: `src/lib/toast.ts` (`showError`)
- Create: `src/components/Toasts.tsx` (`<Toasts />`: `ToastContainer` + 다운로드·업데이트 토스트 동기화)
- Delete: `src/components/ErrorBar.tsx`, `src/components/DownloadToast.tsx`
- Modify: `src/lib/settings.ts` (`error/setError/clearError` 제거, load 실패는 `showError`)
- Modify: `src/lib/update.ts` (`fail` → `showError`)
- Modify: `src/lib/models.ts` (`report` → `showError`), `src/lib/session.ts` (`report` 가 `showError` 사용 — session.ts 의 report 정의 위치를 확인해 같은 방식으로)
- Modify: `src/main.tsx` (오버레이 창이 아니면 `<Toasts />` 렌더)
- Modify: `src/pages/MainApp.tsx`, `src/pages/Onboarding.tsx` (`ErrorBar`/`DownloadToast` 제거, `setError` → `showError`)
- Modify: `src/components/PermissionRow.tsx`, `src/pages/OverlayWindow.tsx`, `src/pages/settings/Overlay.tsx`, `src/pages/settings/General.tsx`, `src/pages/main/History.tsx`, `src/pages/settings/Translation.tsx` (호출부 교체·인라인 알림 제거)
- Modify: `src/index.css` (toastify CSS 변수를 daisyUI 색에 매핑)
- Modify: `src/test/settings.test.ts`, `src/test/update.test.ts` (오류 단언을 mocked `toast.error` 호출로)
- Create: `src/test/toast.test.ts`

**Interfaces:**
- Produces: `showError(e: unknown): void` — `Error` 면 `.message`, 아니면 `String(e)` 로 `toast.error(text)`. 모든 기존 `setError`/`report` 호출부가 이걸 쓴다.
- `useSettings` 에서 `error`, `setError`, `clearError` 가 사라진다.
- 토스트 id 상수: `"download"`, `"update-available"`, `"update-install"` (Toasts.tsx 내부).

- [ ] **Step 1: 의존성**

```bash
yarn add react-toastify
```
v11 이상이어야 한다(CSS 자동 주입). `package.json` 에 `"react-toastify": "^11"` 이 들어갔는지 확인.

- [ ] **Step 2: 실패하는 테스트 — showError**

`src/test/toast.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { toast } from "react-toastify";
import { showError } from "../lib/toast";

vi.mock("react-toastify", () => ({ toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn(), info: vi.fn() }) }));

beforeEach(() => vi.clearAllMocks());

describe("showError", () => {
  it("uses Error.message", () => {
    showError(new Error("disk full"));
    expect(toast.error).toHaveBeenCalledWith("disk full");
  });
  it("stringifies non-Error values", () => {
    showError("busy_stopping");
    expect(toast.error).toHaveBeenCalledWith("busy_stopping");
  });
});
```

Run: `yarn test src/test/toast.test.ts` → FAIL (모듈 없음)

- [ ] **Step 3: showError**

`src/lib/toast.ts`:

```ts
import { toast } from "react-toastify";

// 오류 알림의 단일 입구. 스토어와 페이지는 이 함수만 알고 토스트 라이브러리는 모른다.
export const showError = (e: unknown) => {
  toast.error(e instanceof Error ? e.message : String(e));
};
```

Run: `yarn test src/test/toast.test.ts` → 2 passed

- [ ] **Step 4: 스토어에서 error 상태 제거**

`src/lib/settings.ts`:
- `import { showError } from "./toast";`
- `SettingsStore` 에서 `error`, `setError`, `clearError` 세 줄 삭제.
- 구현에서 `error: null,`, `setError: ...`, `clearError: ...` 삭제.
- `load` 의 catch: `set({ settings: get().settings ?? defaultSettings }); showError(e);`
- `update` 의 catch: `set({ settings: prev }); showError(e);`
- `message` 헬퍼가 더 이상 안 쓰이면 삭제.

`src/lib/update.ts`: `import { useSettings } from "./settings";` 를 `import { showError } from "./toast";` 로, `const fail = (e: unknown) => useSettings.getState().setError(e);` 를 `const fail = showError;` 로.

`src/lib/models.ts` 와 `src/lib/session.ts` 의 `report`: `useSettings.getState().setError(...)` 를 `showError(...)` 로(각 파일에 `import { showError } from "./toast";`). `useSettings` import 가 그 파일에서 다른 용도로 안 쓰이면 지운다.

- [ ] **Step 5: 기존 테스트 갱신**

`src/test/settings.test.ts` 와 `src/test/update.test.ts`:
- 파일 상단에 `vi.mock("react-toastify", () => ({ toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn(), info: vi.fn(), update: vi.fn(), dismiss: vi.fn(), isActive: vi.fn(() => false) }) }));` 와 `import { toast } from "react-toastify";` 추가.
- `useSettings.setState({ ..., error: null })` 에서 `error: null` 제거. `expect(useSettings.getState().error).toBe("disk full")` → `expect(toast.error).toHaveBeenCalledWith("disk full")`. `expect(...error).toBeNull()` → `expect(toast.error).not.toHaveBeenCalled()`. `beforeEach` 에 `vi.clearAllMocks()` (또는 `vi.mocked(toast.error).mockClear()`) 추가.
- update.test.ts 의 `useSettings` import 가 더 이상 안 쓰이면 제거.

Run: `yarn test src/test/settings.test.ts src/test/update.test.ts` → 모두 통과

- [ ] **Step 6: 호출부 교체**

각 파일에서 `useSettings` 의 `setError` 사용을 `showError` 로:
- `src/components/PermissionRow.tsx`: `const setError = useSettings((s) => s.setError);` 삭제, `.catch(setError)` → `.catch(showError)` (2곳). import 교체.
- `src/pages/OverlayWindow.tsx`: `.catch(useSettings.getState().setError)` → `.catch(showError)` (2곳). `useSettings` 는 settings 읽기에 계속 쓰이므로 import 유지, `showError` import 추가.
- `src/pages/settings/Overlay.tsx`: `const { settings, update, setError } = useSettings();` → `const { settings, update } = useSettings();`, `setError` → `showError` (2곳).
- `src/pages/settings/General.tsx`: `const setError = useSettings((s) => s.setError);` 삭제, `setError(e)` → `showError(e)`. 또한 설치 중 인라인 `<progress>` 를 없애고 `{t("update.installing")}` 텍스트만 남긴다(진행률은 토스트가 보여준다).
- `src/pages/Onboarding.tsx`: `const { settings, update, setError } = useSettings();` → `const { settings, update } = useSettings();`, `.catch(setError)` → `.catch(showError)`, `<DownloadToast />` 와 `<ErrorBar />` 줄과 import 제거.
- `src/pages/MainApp.tsx`: `<DownloadToast />`, `<ErrorBar />` 줄과 import 제거.
- `src/pages/main/History.tsx`: `setError` → `showError` (fail 안), `saved` state·`toastTimer`·`toast` 함수·`useEffect(() => () => window.clearTimeout(...))`·`{saved && ...}` 렌더 삭제. `exportAs` 는 `.then((path) => toast.success(t("history.saved", { path })))` (`import { toast } from "react-toastify";`). `useRef` import 가 남지 않으면 정리.
- `src/pages/settings/Translation.tsx`: `result` state 와 인라인 `<div role="status" className="alert ...">` 삭제. `test()` 는 성공이면 `toast.success(t("translation.testResult", { ms: r.ms, text: r.text }))`, 실패면 `toast.error(failText(r.error, r.text))`, catch 는 `toast.error(failText(m, ""))`. `report` 가 그 파일에서 `setError` 였다면 `showError` 로.
- `src/components/ErrorBar.tsx`, `src/components/DownloadToast.tsx` 삭제 (`git rm`).

Run: `yarn tsc --noEmit` → `setError`/`error` 참조가 남아 있으면 여기서 잡힌다. 모두 0 이 될 때까지.

- [ ] **Step 7: Toasts 컴포넌트**

`src/components/Toasts.tsx`:

```tsx
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ToastContainer, toast } from "react-toastify";
import { formatSize, useModels } from "../lib/models";
import { useUpdate } from "../lib/update";

const DOWNLOAD = "download";
const UPDATE_AVAILABLE = "update-available";
const UPDATE_INSTALL = "update-install";

// 받는 중인 모델 하나 + 대기 목록. 토스트 본문은 스토어를 직접 구독하므로 toast.update 로 다시 그릴 필요가 없다.
function DownloadBody() {
  const { t } = useTranslation();
  const models = useModels((s) => s.models);
  const queue = useModels((s) => s.queue);
  const cancel = useModels((s) => s.cancel);
  const dequeue = useModels((s) => s.dequeue);
  const active = models.find((m) => m.download);
  const name = (id: string) => models.find((m) => m.info.id === id)?.info.name ?? id;
  return (
    <div className="text-sm">
      {active?.download && (
        <div className="flex items-center justify-between gap-2">
          <span className="truncate font-semibold">{t("downloads.downloading", { name: active.info.name })}</span>
          <button type="button" className="btn btn-ghost btn-xs" aria-label={t("models.cancel")} onClick={() => cancel(active.info.id)}>✕</button>
        </div>
      )}
      {active?.download && (
        <div className="text-xs opacity-70">
          {Math.round((active.download.received / Math.max(1, active.download.total)) * 100)}% · {formatSize(active.download.received)} / {formatSize(active.download.total)}
        </div>
      )}
      {queue.map((id) => (
        <div key={id} className="mt-1 flex items-center justify-between gap-2 text-xs opacity-70">
          <span className="truncate">{t("downloads.next", { name: name(id) })}</span>
          <button type="button" className="btn btn-ghost btn-xs" aria-label={t("models.cancel")} onClick={() => dequeue(id)}>✕</button>
        </div>
      ))}
    </div>
  );
}

function UpdateAvailableBody({ version, closeToast }: { version: string; closeToast?: () => void }) {
  const { t } = useTranslation();
  const install = useUpdate((s) => s.install);
  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <span>{t("update.available", { version })}</span>
      <button type="button" className="btn btn-primary btn-xs" onClick={() => { closeToast?.(); install(); }}>{t("update.install")}</button>
    </div>
  );
}

// 스토어 상태를 토스트로 비춘다. 스토어는 토스트를 모른다.
export function Toasts() {
  const { t } = useTranslation();
  const downloading = useModels((s) => s.models.some((m) => m.download) || s.queue.length > 0);
  const received = useModels((s) => s.models.find((m) => m.download)?.download?.received ?? 0);
  const total = useModels((s) => s.models.find((m) => m.download)?.download?.total ?? 0);
  const info = useUpdate((s) => s.info);
  const progress = useUpdate((s) => s.progress);

  useEffect(() => {
    if (!downloading) { toast.dismiss(DOWNLOAD); return; }
    const p = total > 0 ? received / total : 0;
    if (toast.isActive(DOWNLOAD)) toast.update(DOWNLOAD, { progress: p });
    else toast(<DownloadBody />, { toastId: DOWNLOAD, autoClose: false, closeButton: false, closeOnClick: false, draggable: false, progress: p });
  }, [downloading, received, total]);

  useEffect(() => {
    if (!info) { toast.dismiss(UPDATE_AVAILABLE); return; }
    if (!toast.isActive(UPDATE_AVAILABLE)) toast.info(<UpdateAvailableBody version={info.version} />, { toastId: UPDATE_AVAILABLE, autoClose: false, closeOnClick: false });
  }, [info]);

  useEffect(() => {
    if (!progress) { toast.dismiss(UPDATE_INSTALL); return; }
    const p = progress.total ? progress.received / progress.total : 0;
    if (toast.isActive(UPDATE_INSTALL)) toast.update(UPDATE_INSTALL, { progress: p });
    else toast(t("update.installing"), { toastId: UPDATE_INSTALL, autoClose: false, closeButton: false, closeOnClick: false, draggable: false, progress: p });
  }, [progress, t]);

  return <ToastContainer position="top-right" theme="light" newestOnTop closeOnClick pauseOnFocusLoss={false} />;
}
```

`react-toastify` 에서 커스텀 본문 컴포넌트는 `closeToast` prop 을 받는다(`toast(<Comp />)` 로 넘긴 엘리먼트에 주입). 타입이 맞지 않으면 `toast.info(({ closeToast }) => <UpdateAvailableBody version={info.version} closeToast={closeToast} />, ...)` 렌더 함수 형태로 바꾼다.

`src/main.tsx`: `import { Toasts } from "./components/Toasts";` 추가, Root 의 return 을

```tsx
  return (
    <React.Suspense fallback={null}>
      {page}
      {label !== "overlay" && <Toasts />}
    </React.Suspense>
  );
```

- [ ] **Step 8: 테마 매핑**

`src/index.css` 의 daisyUI 테마 블록들 아래에:

```css
/* react-toastify 는 theme="light" 로 고정하고 색만 daisyUI 변수에 붙인다 → 다크/라이트를 자동으로 따라간다. */
:root {
  --toastify-color-light: var(--color-neutral);
  --toastify-text-color-light: var(--color-neutral-content);
  --toastify-color-info: var(--color-info);
  --toastify-color-success: var(--color-success);
  --toastify-color-warning: var(--color-warning);
  --toastify-color-error: var(--color-error);
  --toastify-color-progress-light: var(--color-primary);
  --toastify-toast-bd-radius: var(--radius-box);
  --toastify-font-family: inherit;
}
```

- [ ] **Step 9: 게이트와 눈 확인**

Run: `yarn tsc --noEmit && yarn test`
Expected: 통과. `grep -rn "setError\|ErrorBar\|DownloadToast\|clearError" src` 가 비어야 한다.

Run: `mise exec -- yarn tauri dev`
확인: (1) 설정 › 일반 "지금 확인" → 오른쪽 위 빨간 토스트(오류). (2) 모델 탭에서 작은 모델 하나 다운로드 시작 → 진행 토스트(퍼센트, 취소 버튼), 취소하면 사라짐. (3) 히스토리에서 세션 하나 TXT 내보내기 → 초록 토스트 "저장됨: 경로". (4) 번역 탭 "연결 테스트" → 결과 토스트(성공 초록/실패 빨강), 인라인 alert 없음. (5) 다크/라이트 전환 시 토스트 배경·글자색이 따라감. 온보딩 창은 `settings.json` 의 `onboarding_done` 을 잠깐 false 로 바꿔 띄워 보고 오류 토스트가 뜨는지 확인한 뒤 되돌린다(또는 생략하고 보고에 적는다).

- [ ] **Step 10: 커밋**

```bash
git add -A src package.json yarn.lock
git commit -m "feat(ui): 알림을 react-toastify 로 통일(오류·다운로드·내보내기·번역 테스트·업데이트)"
```
