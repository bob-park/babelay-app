# Babelay 1단계: 앱 셸 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 엔진 없이도 동작하는 Babelay 앱 셸을 만든다. 테마·i18n·설정 파일·접이식 사이드바·트레이·전역 단축키·온보딩 골격·오버레이 창(조정 모드)·아이콘·서명 설정·CI까지.

**Architecture:** Tauri 2 단일 프로세스. Cargo 워크스페이스에 `src-tauri`(앱)와 `crates/babelay-engine`(2단계에서 채울 빈 라이브러리)을 둔다. 프론트엔드는 Vite 앱 하나이고, 창 라벨(`main`/`overlay`/`onboarding`)로 무엇을 렌더할지 정한다. 설정은 Rust가 `settings.json`으로 소유하고, 프론트는 `get_settings`/`set_settings` 커맨드와 `settings-changed` 이벤트로 동기화한다.

**Tech Stack:** Rust 1.98, Tauri 2.11 (`tray-icon`, `macos-private-api`), tauri-plugin-global-shortcut 2, tauri-plugin-opener 2, serde/serde_json, sys-locale 0.3, yarn 4.18.1 + Node 24 (`.mise.toml`로 고정), React 19, TypeScript 5.9, Vite, Tailwind 4 (`@tailwindcss/vite`), react-router 7, zustand 5, i18next 26 + react-i18next 17, vitest 4, sharp(아이콘 생성 전용 devDependency).

**Spec:** `docs/superpowers/specs/2026-09-02-babelay-design.md` (섹션 3, 7, 9, 11의 1단계)

## Global Constraints

- 대상 OS: macOS 14.2 이상(Apple Silicon), Windows 10 이상(x64). `LSMinimumSystemVersion=14.2`.
- 앱 식별자 `com.babelay.app`, 제품명 `Babelay`.
- UI 라이브러리 금지. Tailwind + 네이티브 요소(`<select>`, `<dialog>`, `<input type=range>`)만 사용.
- 지원 언어 `ko`, `en`, `ja`. 설정값 `system`은 시스템 로케일로 해석하고, 지원 밖은 `en`.
- 테마 `system|dark|light`. 다크 팔레트는 `docs/design/spotify-design.md` 값(`#121212`, `#181818`, `#1f1f1f`, `#b3b3b3`, `#1ed760`). 라이트는 배경 `#ffffff`/`#f5f5f5`, 표면 `#eeeeee`, 텍스트 `#121212`. 초록은 "채우기 + 검정 글자"로만 쓰고 흰 배경 위 초록 글자는 금지.
- 설정 파일 스키마는 스펙 7.6과 동일한 snake_case 키를 쓴다.
- 오버레이 위치는 `{monitor_id, x_ratio, y_ratio, w_ratio}` 비율로 저장한다.
- 전역 단축키 고정값: `CmdOrCtrl+Shift+S` 캡처 토글, `CmdOrCtrl+Shift+O` 오버레이 토글.
- 커밋 메시지는 `feat:`/`chore:`/`test:`/`ci:` 접두어를 쓰고, 끝에 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` 줄을 붙인다.
- 각 태스크의 검증 명령은 저장소 루트에서 실행한다.
- Node/yarn 버전은 `.mise.toml`(node 24, yarn 4.18.1)이 결정한다. 셸에 mise가 활성화되어 있지 않으면 `mise exec -- yarn …`로 실행한다. `yarn --version`이 `4.18.1`이 아니면 진행하지 않는다.

---

## 파일 구조

```
babelay-app/
├─ Cargo.toml                       # 워크스페이스 루트
├─ package.json, yarn.lock
├─ vite.config.ts, tsconfig.json, index.html
├─ assets/
│  ├─ icon.svg                      # 앱 아이콘 원본
│  └─ tray.svg                      # 트레이 아이콘 원본(검정)
├─ scripts/gen-icons.mjs            # tray.svg → PNG
├─ crates/babelay-engine/           # 빈 라이브러리(2단계에서 채움)
├─ src-tauri/
│  ├─ Cargo.toml, tauri.conf.json, build.rs
│  ├─ Info.plist, entitlements.plist
│  ├─ capabilities/default.json
│  ├─ icons/                        # tauri icon 생성물 + tray-*.png
│  └─ src/
│     ├─ main.rs                    # babelay_lib::run()
│     ├─ lib.rs                     # Builder 조립, setup
│     ├─ settings.rs                # Settings 구조체, load/save, SettingsState
│     ├─ i18n.rs                    # Lang, resolve, 트레이 라벨
│     ├─ windows.rs                 # main/onboarding 창 생성·표시
│     ├─ overlay.rs                 # 오버레이 창 생성, 비율 ↔ 좌표, 모니터 목록
│     ├─ tray.rs                    # 트레이 메뉴, 전역 단축키, 토글 동작
│     └─ commands.rs                # #[tauri::command] 모음
└─ src/
   ├─ main.tsx                      # 창 라벨로 분기
   ├─ index.css                     # Tailwind + 팔레트 토큰
   ├─ lib/
   │  ├─ types.ts                   # Settings TS 타입
   │  ├─ tauri.ts                   # invoke 래퍼
   │  ├─ settings.ts                # zustand 설정 스토어 + mergeSettings
   │  ├─ session.ts                 # capturing 플래그 스토어
   │  ├─ i18n.ts                    # i18next 초기화 + resolveLang
   │  ├─ theme.ts                   # resolveTheme + applyTheme
   │  └─ models.fixture.ts          # 모델 목록 임시 데이터(2단계에서 교체)
   ├─ locales/{ko,en,ja}.json
   ├─ components/{Sidebar,Badge,PillButton,Toggle,ModelRow}.tsx
   ├─ pages/
   │  ├─ MainApp.tsx                # HashRouter + Sidebar 레이아웃
   │  ├─ main/{Live,History}.tsx
   │  ├─ settings/{General,Transcription,Translation,Overlay}.tsx
   │  ├─ OverlayWindow.tsx
   │  └─ Onboarding.tsx
   └─ test/{settings,theme,i18n,locales}.test.ts
```

---

### Task 1: 프로젝트 스캐폴드와 워크스페이스

**Files:**
- Create: `Cargo.toml`, `crates/babelay-engine/{Cargo.toml,src/lib.rs}`, `package.json`, `vite.config.ts`, `index.html`, `src/main.tsx`, `src/index.css`, `src-tauri/**`, `.github/` 없음(Task 10)
- Modify: `.gitignore`

**Interfaces:**
- Produces: `yarn tauri dev`로 뜨는 빈 창, `yarn test`, `cargo test --workspace`가 통과하는 상태. 패키지 이름 `babelay`, 라이브러리 크레이트 `babelay_lib`.

- [ ] **Step 1: Tauri 템플릿을 임시 디렉터리에 생성하고 저장소 루트로 옮긴다**

```bash
cd /tmp && rm -rf babelay-scaffold
npx --yes create-tauri-app@latest babelay-scaffold --template react-ts --manager yarn --identifier com.babelay.app --yes
rsync -a --exclude .git /tmp/babelay-scaffold/ /Users/hwpark/Documents/rust-workspace/babelay-app/
cd /Users/hwpark/Documents/rust-workspace/babelay-app && ls
```

Expected: `package.json`, `src/`, `src-tauri/`, `vite.config.ts`가 루트에 생김.

- [ ] **Step 2: yarn 4 확인과 node_modules 링커 설정**

yarn은 `.mise.toml`이 4.18.1로 고정한다. PnP 대신 node_modules 링커를 쓴다(Vite·sharp 호환).

```bash
npm pkg set packageManager=yarn@4.18.1
yarn --version
yarn config set nodeLinker node-modules
```

`packageManager` 필드는 corepack 셸이 남아 있는 환경에서도 yarn 1이 아닌 4.18.1을 고르게 한다.

Expected: `4.18.1`이 출력되고, 루트에 `.yarnrc.yml`(`nodeLinker: node-modules`)이 생긴다. 템플릿이 만든 `yarn.lock`(v1 형식)이 있으면 지우고 다음 단계의 `yarn add`가 새로 만들게 둔다.

- [ ] **Step 3: 패키지 이름과 버전 고정**

`package.json`의 `name`을 `babelay`로, `src-tauri/Cargo.toml`의 `[package] name`을 `babelay`로, `[lib] name`을 `babelay_lib`로 맞춘다. `src-tauri/src/main.rs`가 `babelay_lib::run()`을 부르는지 확인한다.

```bash
yarn add react-router@^7 zustand@^5 i18next@^26 react-i18next@^17 @tauri-apps/plugin-global-shortcut@^2 @tauri-apps/plugin-opener@^2
yarn add -D tailwindcss@^4 @tailwindcss/vite@^4 vitest@^4 typescript@^5.9 sharp
```

- [ ] **Step 4: 워크스페이스 Cargo.toml과 빈 엔진 크레이트**

`Cargo.toml` (루트):

```toml
[workspace]
members = ["src-tauri", "crates/babelay-engine"]
resolver = "2"
```

```bash
cargo new crates/babelay-engine --lib --name babelay-engine --vcs none
```

`crates/babelay-engine/src/lib.rs`:

```rust
//! Babelay 엔진. 오디오 캡처·전사·번역은 2단계에서 채운다.

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_not_empty() {
        assert!(!super::version().is_empty());
    }
}
```

- [ ] **Step 5: Tailwind 4 연결**

`vite.config.ts`:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: { environment: "node", include: ["src/test/**/*.test.ts"] },
}));
```

`src/index.css` (템플릿의 App.css는 삭제):

```css
@import "tailwindcss";
@custom-variant dark (&:where(.dark, .dark *));

@theme {
  --color-base: #ffffff;
  --color-base-2: #f5f5f5;
  --color-surface: #eeeeee;
  --color-surface-2: #e4e4e4;
  --color-fg: #121212;
  --color-fg-muted: #5a5a5a;
  --color-accent: #1ed760;
  --color-accent-fg: #000000;
  --color-danger: #d33a4a;
  --font-sans: -apple-system, "Helvetica Neue", Arial, "Hiragino Sans", "Apple SD Gothic Neo", "Meiryo", sans-serif;
}

.dark {
  --color-base: #121212;
  --color-base-2: #181818;
  --color-surface: #1f1f1f;
  --color-surface-2: #252525;
  --color-fg: #ffffff;
  --color-fg-muted: #b3b3b3;
  --color-danger: #f3727f;
}

html, body, #root { height: 100%; margin: 0; }
body { background: var(--color-base); color: var(--color-fg); font-family: var(--font-sans); font-size: 14px; }
```

`src/main.tsx`를 최소로 교체한다 (라벨 분기는 Task 6):

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <div className="p-4">Babelay</div>
  </React.StrictMode>,
);
```

템플릿의 `src/App.tsx`, `src/App.css`, `src/assets/`는 삭제한다. `package.json`의 `scripts`에 `"test": "vitest run"`을 추가한다.

- [ ] **Step 6: .gitignore 정리**

루트 `.gitignore`에서 `Cargo.lock` 줄을 지우고(실행 파일이므로 잠금 파일을 커밋한다), 아래를 추가한다:

```
node_modules/
.yarn/
.pnp.*
dist/
src-tauri/gen/
```

- [ ] **Step 7: 빌드와 테스트 확인**

```bash
yarn install
cargo test --workspace
yarn test
yarn tauri build --debug --no-bundle
```

Expected: `cargo test`는 `version_is_not_empty` 통과, `yarn test`는 "No test files found"가 아닌 종료 코드 0(테스트 파일이 없어도 `vitest run --passWithNoTests`가 필요하면 `scripts.test`를 `"vitest run --passWithNoTests"`로 둔다), `tauri build --debug --no-bundle`은 컴파일 성공.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore: scaffold tauri 2 + react + tailwind workspace"
```

---

### Task 2: 앱 아이콘과 트레이 아이콘

**Files:**
- Create: `assets/icon.svg`, `assets/tray.svg`, `scripts/gen-icons.mjs`, `src-tauri/icons/*`

**Interfaces:**
- Produces: `src-tauri/icons/icon.icns`, `icon.ico`, `32x32.png`, `128x128.png` 등(`tauri icon` 생성물), `src-tauri/icons/tray-22.png`, `tray-44.png`(검정, macOS 템플릿), `tray-win-32.png`(흰색, Windows).

- [ ] **Step 1: 아이콘 SVG 작성** (목업 05의 C 시안)

`assets/icon.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#2a2a2a"/>
      <stop offset="1" stop-color="#121212"/>
    </linearGradient>
  </defs>
  <rect x="100" y="100" width="824" height="824" rx="185" fill="url(#bg)"/>
  <rect x="384" y="282" width="256" height="76" rx="38" fill="#1ed760"/>
  <rect x="332" y="410" width="360" height="76" rx="38" fill="#ffffff"/>
  <rect x="280" y="538" width="464" height="76" rx="38" fill="#b3b3b3"/>
  <rect x="228" y="666" width="568" height="76" rx="38" fill="#6a6a6a"/>
</svg>
```

`assets/tray.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 22 22">
  <g fill="#000000">
    <rect x="8" y="4" width="6" height="2.2" rx="1.1"/>
    <rect x="6.5" y="8" width="9" height="2.2" rx="1.1"/>
    <rect x="5" y="12" width="12" height="2.2" rx="1.1"/>
    <rect x="3.5" y="16" width="15" height="2.2" rx="1.1"/>
  </g>
</svg>
```

- [ ] **Step 2: 트레이 PNG 생성 스크립트**

`scripts/gen-icons.mjs`:

```js
import sharp from "sharp";
import { readFileSync } from "node:fs";

const svg = readFileSync(new URL("../assets/tray.svg", import.meta.url), "utf8");
const white = svg.replace('fill="#000000"', 'fill="#ffffff"');
const out = (n) => new URL(`../src-tauri/icons/${n}`, import.meta.url).pathname;

await sharp(Buffer.from(svg)).resize(22, 22).png().toFile(out("tray-22.png"));
await sharp(Buffer.from(svg)).resize(44, 44).png().toFile(out("tray-44.png"));
await sharp(Buffer.from(white)).resize(32, 32).png().toFile(out("tray-win-32.png"));
console.log("tray icons written");
```

`package.json` scripts에 `"icons": "tauri icon assets/icon.svg && node scripts/gen-icons.mjs"` 추가.

- [ ] **Step 3: 생성 실행**

```bash
yarn icons
ls src-tauri/icons
```

Expected: `icon.icns`, `icon.ico`, `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png`, `tray-22.png`, `tray-44.png`, `tray-win-32.png`가 있음. `file src-tauri/icons/tray-22.png`가 `22 x 22` PNG를 보고한다.

- [ ] **Step 4: Commit**

```bash
git add assets scripts src-tauri/icons package.json
git commit -m "feat: add app and tray icons"
```

---

### Task 3: 설정 모듈 (Rust)

**Files:**
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`

**Interfaces:**
- Produces:
  - `settings::Settings` (serde, `Default`, `PartialEq`), 하위 `General`, `Asr`, `Translation`, `Cloud`, `Overlay`
  - `Settings::load(path: &Path) -> Settings`, `Settings::save(&self, path: &Path) -> io::Result<()>`
  - `settings::SettingsState { path: PathBuf, current: Mutex<Settings> }`, `SettingsState::get(&self) -> Settings`, `SettingsState::set(&self, app: &AppHandle, new: Settings) -> Result<(), String>` (저장 + `settings-changed` 이벤트 emit)

- [ ] **Step 1: 의존성 추가**

`src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "macos-private-api", "image-png"] }
tauri-plugin-global-shortcut = "2"
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sys-locale = "0.3"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: 실패하는 테스트 작성**

`src-tauri/src/settings.rs` 하단:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = Settings::default();
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
    }

    #[test]
    fn missing_fields_are_filled_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"version":1,"general":{"theme":"dark"}}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.general.theme, "dark");
        assert_eq!(s.general.ui_language, "system");
        assert_eq!(s.overlay.font_size, 24);
    }

    #[test]
    fn corrupt_file_falls_back_to_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn missing_file_is_default() {
        assert_eq!(Settings::load(Path::new("/nonexistent/settings.json")), Settings::default());
    }
}
```

- [ ] **Step 3: 테스트가 실패하는지 확인**

`src-tauri/src/lib.rs` 상단에 `mod settings;`를 추가한 뒤:

```bash
cargo test -p babelay
```

Expected: `Settings` 미정의로 컴파일 실패.

- [ ] **Step 4: 구현**

`src-tauri/src/settings.rs` 상단(테스트 모듈 위):

```rust
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter};

// ponytail: 열거형 대신 String. 프론트 TS 유니온이 값을 제한하고,
// 모르는 값은 각 소비처에서 기본값으로 취급한다.

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub general: General,
    pub asr: Asr,
    pub translation: Translation,
    pub overlay: Overlay,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct General {
    pub theme: String,       // system | dark | light
    pub ui_language: String, // system | ko | en | ja
    pub onboarding_done: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Asr {
    pub model_id: String,
    pub gpu: bool,
    pub source_lang: String, // auto | ko | en | ja
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Translation {
    pub backend: String, // local | cloud
    pub local_model: String,
    pub cloud: Cloud,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Cloud {
    pub provider: String, // openai | anthropic | gemini | deepl | custom
    pub model: String,
    pub base_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Overlay {
    pub enabled: bool,
    pub monitor_id: String, // "" = 주 모니터
    pub x_ratio: f64,
    pub y_ratio: f64,
    pub w_ratio: f64,
    pub display_mode: String, // both | source | target
    pub subtitle_lang: String, // system | ko | en | ja
    pub font_size: u32,
    pub bg_opacity: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            general: General::default(),
            asr: Asr::default(),
            translation: Translation::default(),
            overlay: Overlay::default(),
        }
    }
}

impl Default for General {
    fn default() -> Self {
        Self { theme: "system".into(), ui_language: "system".into(), onboarding_done: false }
    }
}

impl Default for Asr {
    fn default() -> Self {
        Self { model_id: "small".into(), gpu: true, source_lang: "auto".into() }
    }
}

impl Default for Translation {
    fn default() -> Self {
        Self { backend: "local".into(), local_model: "qwen3.5-2b".into(), cloud: Cloud::default() }
    }
}

impl Default for Cloud {
    fn default() -> Self {
        Self { provider: "openai".into(), model: "gpt-4o-mini".into(), base_url: String::new() }
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            enabled: true,
            monitor_id: String::new(),
            x_ratio: 0.5,
            y_ratio: 0.85,
            w_ratio: 0.6,
            display_mode: "both".into(),
            subtitle_lang: "system".into(),
            font_size: 24,
            bg_opacity: 0.8,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("settings: parse error, using defaults: {e}");
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(&tmp, text)?;
        fs::rename(tmp, path)
    }
}

pub struct SettingsState {
    pub path: PathBuf,
    pub current: Mutex<Settings>,
}

impl SettingsState {
    pub fn new(path: PathBuf) -> Self {
        let current = Mutex::new(Settings::load(&path));
        Self { path, current }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    pub fn set(&self, app: &AppHandle, new: Settings) -> Result<(), String> {
        new.save(&self.path).map_err(|e| e.to_string())?;
        *self.current.lock().unwrap() = new.clone();
        app.emit("settings-changed", &new).map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 5: 테스트 통과 확인**

```bash
cargo test -p babelay settings
```

Expected: 4개 테스트 PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: settings model with json load/save"
```

---

### Task 4: 로케일 해석과 트레이 라벨 (Rust)

**Files:**
- Create: `src-tauri/src/i18n.rs`
- Modify: `src-tauri/src/lib.rs` (`mod i18n;`)

**Interfaces:**
- Produces: `i18n::Lang { Ko, En, Ja }`, `i18n::resolve(pref: &str) -> Lang`, `i18n::resolve_with(pref: &str, system: Option<&str>) -> Lang`, `i18n::TrayLabels { start, stop, overlay_on, overlay_off, open, quit }: &'static str`, `i18n::tray_labels(lang: Lang) -> TrayLabels`

- [ ] **Step 1: 실패하는 테스트**

`src-tauri/src/i18n.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_pref_wins() {
        assert_eq!(resolve_with("ja", Some("ko-KR")), Lang::Ja);
    }

    #[test]
    fn system_uses_locale_prefix() {
        assert_eq!(resolve_with("system", Some("ko-KR")), Lang::Ko);
        assert_eq!(resolve_with("system", Some("ja")), Lang::Ja);
        assert_eq!(resolve_with("system", Some("en-US")), Lang::En);
    }

    #[test]
    fn unsupported_falls_back_to_english() {
        assert_eq!(resolve_with("system", Some("de-DE")), Lang::En);
        assert_eq!(resolve_with("system", None), Lang::En);
        assert_eq!(resolve_with("zz", Some("ko")), Lang::En);
    }

    #[test]
    fn tray_labels_are_localized() {
        assert_eq!(tray_labels(Lang::Ko).quit, "종료");
        assert_eq!(tray_labels(Lang::En).quit, "Quit");
        assert_eq!(tray_labels(Lang::Ja).quit, "終了");
    }
}
```

- [ ] **Step 2: 실패 확인**

```bash
cargo test -p babelay i18n
```

Expected: 컴파일 실패(`Lang` 미정의).

- [ ] **Step 3: 구현**

`src-tauri/src/i18n.rs` 상단:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ko,
    En,
    Ja,
}

pub fn resolve(pref: &str) -> Lang {
    let system = sys_locale::get_locale();
    resolve_with(pref, system.as_deref())
}

pub fn resolve_with(pref: &str, system: Option<&str>) -> Lang {
    let code = if pref == "system" { system.unwrap_or("en") } else { pref };
    let primary = code.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    match primary.as_str() {
        "ko" => Lang::Ko,
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}

pub struct TrayLabels {
    pub start: &'static str,
    pub stop: &'static str,
    pub overlay_on: &'static str,
    pub overlay_off: &'static str,
    pub open: &'static str,
    pub quit: &'static str,
}

pub fn tray_labels(lang: Lang) -> TrayLabels {
    match lang {
        Lang::Ko => TrayLabels {
            start: "캡처 시작",
            stop: "캡처 정지",
            overlay_on: "오버레이 켜기",
            overlay_off: "오버레이 끄기",
            open: "Babelay 열기",
            quit: "종료",
        },
        Lang::En => TrayLabels {
            start: "Start Capture",
            stop: "Stop Capture",
            overlay_on: "Show Overlay",
            overlay_off: "Hide Overlay",
            open: "Open Babelay",
            quit: "Quit",
        },
        Lang::Ja => TrayLabels {
            start: "キャプチャ開始",
            stop: "キャプチャ停止",
            overlay_on: "オーバーレイを表示",
            overlay_off: "オーバーレイを非表示",
            open: "Babelay を開く",
            quit: "終了",
        },
    }
}
```

- [ ] **Step 4: 통과 확인**

```bash
cargo test -p babelay i18n
```

Expected: 4개 PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/i18n.rs src-tauri/src/lib.rs
git commit -m "feat: locale resolution and tray labels"
```

---

### Task 5: 창, 오버레이, 트레이, 단축키, 커맨드 (Rust)

**Files:**
- Create: `src-tauri/src/windows.rs`, `src-tauri/src/overlay.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: `settings::{Settings, SettingsState, Overlay}`, `i18n::{resolve, tray_labels}`
- Produces (프론트가 호출하는 커맨드):
  - `get_settings() -> Settings`
  - `set_settings(settings: Settings) -> Result<(), String>`
  - `get_platform() -> String` (`"macos"` | `"windows"` | 기타 `std::env::consts::OS`)
  - `check_audio_permission() -> String` (`"granted"` | `"denied"` | `"unknown"`) — 1단계는 스텁
  - `open_privacy_settings() -> Result<(), String>`
  - `finish_onboarding() -> Result<(), String>`
  - `overlay_set_adjust_mode(enabled: bool) -> Result<(), String>`
  - `overlay_get_monitors() -> Result<Vec<MonitorInfo>, String>`; `MonitorInfo { id, x, y, width, height, scale, primary }`
  - `overlay_commit_position() -> Result<(), String>`
- Produces (이벤트): `settings-changed`(Settings), `capture-toggle`(없음), `overlay-adjust-mode`(bool)

- [ ] **Step 1: 오버레이 기하 함수의 실패 테스트**

`src-tauri/src/overlay.rs` 하단:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Overlay;

    fn mon() -> Rect {
        Rect { x: 0, y: 0, w: 2000, h: 1000 }
    }

    #[test]
    fn default_ratios_give_bottom_center_rect() {
        let r = rect_from(&Overlay::default(), &mon());
        assert_eq!(r.w, 1200); // 0.6 * 2000
        assert_eq!(r.h, 200); // 0.2 * 1000
        assert_eq!(r.x, 400); // center 1000 - 600
        assert_eq!(r.y, 750); // center 850 - 100
    }

    #[test]
    fn ratios_roundtrip() {
        let o = Overlay { x_ratio: 0.3, y_ratio: 0.2, w_ratio: 0.5, ..Overlay::default() };
        let r = rect_from(&o, &mon());
        let (x, y, w) = ratios_from(&r, &mon());
        assert!((x - 0.3).abs() < 1e-6);
        assert!((y - 0.2).abs() < 1e-6);
        assert!((w - 0.5).abs() < 1e-6);
    }

    #[test]
    fn ratios_are_clamped() {
        let far = Rect { x: -5000, y: 9000, w: 10, h: 10 };
        let (x, y, w) = ratios_from(&far, &mon());
        assert_eq!((x, y, w), (0.0, 1.0, 0.2));
    }

    #[test]
    fn secondary_monitor_offset_is_respected() {
        let m = Rect { x: 2000, y: -500, w: 1000, h: 1000 };
        let r = rect_from(&Overlay::default(), &m);
        assert_eq!(r.x, 2000 + 500 - 300);
        assert_eq!(r.y, -500 + 850 - 100);
    }
}
```

- [ ] **Step 2: 실패 확인**

`lib.rs`에 `mod overlay;`를 추가하고:

```bash
cargo test -p babelay overlay
```

Expected: `Rect`, `rect_from` 미정의로 컴파일 실패.

- [ ] **Step 3: overlay.rs 구현**

```rust
use crate::settings::{Overlay, Settings, SettingsState};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "overlay";
pub static ADJUST_MODE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct MonitorInfo {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}

/// 비율 → 물리 좌표. 높이는 모니터의 20%로 고정한다.
pub fn rect_from(o: &Overlay, mon: &Rect) -> Rect {
    let w = (mon.w as f64 * o.w_ratio.clamp(0.2, 1.0)).round() as u32;
    let h = (mon.h as f64 * 0.2).round() as u32;
    let cx = mon.x as f64 + mon.w as f64 * o.x_ratio.clamp(0.0, 1.0);
    let cy = mon.y as f64 + mon.h as f64 * o.y_ratio.clamp(0.0, 1.0);
    Rect {
        x: (cx - w as f64 / 2.0).round() as i32,
        y: (cy - h as f64 / 2.0).round() as i32,
        w,
        h,
    }
}

/// 물리 좌표 → (x_ratio, y_ratio, w_ratio). 창 중심을 기준으로 한다.
pub fn ratios_from(win: &Rect, mon: &Rect) -> (f64, f64, f64) {
    let cx = win.x as f64 + win.w as f64 / 2.0;
    let cy = win.y as f64 + win.h as f64 / 2.0;
    (
        ((cx - mon.x as f64) / mon.w as f64).clamp(0.0, 1.0),
        ((cy - mon.y as f64) / mon.h as f64).clamp(0.0, 1.0),
        (win.w as f64 / mon.w as f64).clamp(0.2, 1.0),
    )
}

fn monitor_id(m: &tauri::Monitor) -> String {
    m.name()
        .cloned()
        .unwrap_or_else(|| format!("{},{}", m.position().x, m.position().y))
}

fn monitor_rect(m: &tauri::Monitor) -> Rect {
    Rect { x: m.position().x, y: m.position().y, w: m.size().width, h: m.size().height }
}

pub fn monitors(app: &AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let primary = app.primary_monitor().map_err(|e| e.to_string())?.map(|m| monitor_id(&m));
    let list = app.available_monitors().map_err(|e| e.to_string())?;
    Ok(list
        .iter()
        .map(|m| {
            let id = monitor_id(m);
            MonitorInfo {
                primary: Some(&id) == primary.as_ref(),
                id,
                x: m.position().x,
                y: m.position().y,
                width: m.size().width,
                height: m.size().height,
                scale: m.scale_factor(),
            }
        })
        .collect())
}

/// 설정의 monitor_id에 해당하는 모니터. 없으면 주 모니터.
fn target_monitor(app: &AppHandle, id: &str) -> Result<Rect, String> {
    let list = app.available_monitors().map_err(|e| e.to_string())?;
    if let Some(m) = list.iter().find(|m| monitor_id(m) == id) {
        return Ok(monitor_rect(m));
    }
    let primary = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| list.into_iter().next())
        .ok_or("no monitor")?;
    Ok(monitor_rect(&primary))
}

pub fn create(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    if app.get_webview_window(LABEL).is_some() {
        return Ok(());
    }
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("/".into()))
        .title("Babelay Overlay")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    win.set_ignore_cursor_events(true).map_err(|e| e.to_string())?;
    apply_position(app, settings)?;
    if settings.overlay.enabled {
        win.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn apply_position(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    if ADJUST_MODE.load(Ordering::Relaxed) {
        return Ok(()); // 드래그 중에는 설정 반영으로 창을 되돌리지 않는다
    }
    let Some(win) = app.get_webview_window(LABEL) else { return Ok(()) };
    let mon = target_monitor(app, &settings.overlay.monitor_id)?;
    let r = rect_from(&settings.overlay, &mon);
    win.set_size(PhysicalSize::new(r.w, r.h)).map_err(|e| e.to_string())?;
    win.set_position(PhysicalPosition::new(r.x, r.y)).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_visible(app: &AppHandle, visible: bool) -> Result<(), String> {
    let Some(win) = app.get_webview_window(LABEL) else { return Ok(()) };
    if visible { win.show() } else { win.hide() }.map_err(|e| e.to_string())
}

pub fn set_adjust_mode(app: &AppHandle, enabled: bool) -> Result<(), String> {
    ADJUST_MODE.store(enabled, Ordering::Relaxed);
    let Some(win) = app.get_webview_window(LABEL) else { return Ok(()) };
    win.set_ignore_cursor_events(!enabled).map_err(|e| e.to_string())?;
    if enabled {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    }
    app.emit_to(LABEL, "overlay-adjust-mode", enabled).map_err(|e| e.to_string())
}

/// 현재 창 위치·크기를 비율로 환산해 설정에 저장한다.
pub fn commit_position(app: &AppHandle) -> Result<(), String> {
    let Some(win) = app.get_webview_window(LABEL) else { return Ok(()) };
    let pos = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let mon = win
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("no monitor")?;
    let (x, y, w) = ratios_from(
        &Rect { x: pos.x, y: pos.y, w: size.width, h: size.height },
        &monitor_rect(&mon),
    );
    let state = app.state::<SettingsState>();
    let mut s = state.get();
    s.overlay.monitor_id = monitor_id(&mon);
    s.overlay.x_ratio = x;
    s.overlay.y_ratio = y;
    s.overlay.w_ratio = w;
    state.set(app, s)
}
```

- [ ] **Step 4: 기하 테스트 통과 확인**

```bash
cargo test -p babelay overlay
```

Expected: 4개 PASS.

- [ ] **Step 5: windows.rs**

```rust
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const MAIN: &str = "main";
pub const ONBOARDING: &str = "onboarding";

pub fn show_main(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(MAIN) {
        w.show().map_err(|e| e.to_string())?;
        return w.set_focus().map_err(|e| e.to_string());
    }
    let w = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("/".into()))
        .title("Babelay")
        .inner_size(960.0, 640.0)
        .min_inner_size(720.0, 480.0)
        .build()
        .map_err(|e| e.to_string())?;
    // 닫기 = 숨기기. 앱은 트레이에 남는다.
    let handle = w.clone();
    w.on_window_event(move |e| {
        if let WindowEvent::CloseRequested { api, .. } = e {
            api.prevent_close();
            let _ = handle.hide();
        }
    });
    Ok(())
}

pub fn show_onboarding(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(ONBOARDING) {
        return w.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(app, ONBOARDING, WebviewUrl::App("/".into()))
        .title("Babelay")
        .inner_size(720.0, 560.0)
        .resizable(false)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn close_onboarding(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(ONBOARDING) {
        let _ = w.close();
    }
}
```

- [ ] **Step 6: tray.rs**

```rust
use crate::{i18n, overlay, settings::SettingsState, windows};
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const SHORTCUT_CAPTURE: &str = "CmdOrCtrl+Shift+S";
pub const SHORTCUT_OVERLAY: &str = "CmdOrCtrl+Shift+O";

pub fn toggle_capture(app: &AppHandle) {
    // 엔진은 2단계. 지금은 프론트가 이 이벤트로 capturing 플래그를 뒤집는다.
    let _ = app.emit("capture-toggle", ());
}

pub fn toggle_overlay(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<SettingsState>();
    let mut s = state.get();
    s.overlay.enabled = !s.overlay.enabled;
    overlay::set_visible(app, s.overlay.enabled)?;
    state.set(app, s)
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let settings = app.state::<SettingsState>().get();
    let labels = i18n::tray_labels(i18n::resolve(&settings.general.ui_language));

    let capture = MenuItem::with_id(app, "capture", labels.start, true, None::<&str>)?;
    let overlay_item = MenuItem::with_id(app, "overlay", labels.overlay_off, true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", labels.open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", labels.quit, true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .items(&[&capture, &overlay_item])
        .separator()
        .items(&[&open])
        .separator()
        .items(&[&quit])
        .build()?;

    #[cfg(target_os = "macos")]
    let icon = Image::from_bytes(include_bytes!("../icons/tray-22.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = Image::from_bytes(include_bytes!("../icons/tray-win-32.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture" => toggle_capture(app),
            "overlay" => {
                let _ = toggle_overlay(app);
            }
            "open" => {
                let _ = windows::show_main(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    let capture_sc: Shortcut = SHORTCUT_CAPTURE.parse().expect("valid shortcut");
    let overlay_sc: Shortcut = SHORTCUT_OVERLAY.parse().expect("valid shortcut");
    app.global_shortcut().on_shortcut(capture_sc, |app, _, ev| {
        if ev.state() == ShortcutState::Pressed {
            toggle_capture(app);
        }
    })?;
    app.global_shortcut().on_shortcut(overlay_sc, |app, _, ev| {
        if ev.state() == ShortcutState::Pressed {
            let _ = toggle_overlay(app);
        }
    })?;
    Ok(())
}
```

- [ ] **Step 7: commands.rs**

```rust
use crate::{
    overlay,
    settings::{Settings, SettingsState},
    windows,
};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    state.get()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, state: State<'_, SettingsState>, settings: Settings) -> Result<(), String> {
    let before = state.get();
    state.set(&app, settings.clone())?;
    if before.overlay != settings.overlay {
        overlay::apply_position(&app, &settings)?;
        overlay::set_visible(&app, settings.overlay.enabled)?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

// ponytail: 2단계에서 Core Audio 탭 생성 시도로 교체한다.
#[tauri::command]
pub fn check_audio_permission() -> String {
    if cfg!(target_os = "windows") { "granted".into() } else { "unknown".into() }
}

#[tauri::command]
pub fn open_privacy_settings(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let url = if cfg!(target_os = "macos") {
        "x-apple.systempreferences:com.apple.preference.security?Privacy_AudioCapture"
    } else {
        "ms-settings:privacy-microphone"
    };
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn finish_onboarding(app: AppHandle, state: State<'_, SettingsState>) -> Result<(), String> {
    let mut s = state.get();
    s.general.onboarding_done = true;
    state.set(&app, s)?;
    windows::show_main(&app)?;
    windows::close_onboarding(&app);
    Ok(())
}

#[tauri::command]
pub fn overlay_set_adjust_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    overlay::set_adjust_mode(&app, enabled)?;
    if !enabled {
        let s = app.state::<SettingsState>().get();
        overlay::set_visible(&app, s.overlay.enabled)?;
    }
    Ok(())
}

#[tauri::command]
pub fn overlay_get_monitors(app: AppHandle) -> Result<Vec<overlay::MonitorInfo>, String> {
    overlay::monitors(&app)
}

#[tauri::command]
pub fn overlay_commit_position(app: AppHandle) -> Result<(), String> {
    overlay::commit_position(&app)
}
```

- [ ] **Step 8: lib.rs 조립**

```rust
mod commands;
mod i18n;
mod overlay;
mod settings;
mod tray;
mod windows;

use settings::SettingsState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            app.manage(SettingsState::new(path));
            let settings = app.state::<SettingsState>().get();
            let handle = app.handle().clone();
            if settings.general.onboarding_done {
                windows::show_main(&handle)?;
            } else {
                windows::show_onboarding(&handle)?;
            }
            overlay::create(&handle, &settings)?;
            tray::build(&handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::get_platform,
            commands::check_audio_permission,
            commands::open_privacy_settings,
            commands::finish_onboarding,
            commands::overlay_set_adjust_mode,
            commands::overlay_get_monitors,
            commands::overlay_commit_position,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`setup` 클로저는 `Box<dyn Error>`를 반환하므로 `String` 오류는 `.map_err(|e| e.into())`가 필요할 수 있다. 컴파일 오류가 나면 `windows::show_main(&handle).map_err(std::io::Error::other)?;` 형태로 감싼다.

- [ ] **Step 9: tauri.conf.json 수정**

`app.windows` 배열을 비우고(창은 Rust가 만든다), 아래를 맞춘다:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Babelay",
  "version": "0.1.0",
  "identifier": "com.babelay.app",
  "build": {
    "beforeDevCommand": "yarn dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "yarn build",
    "frontendDist": "../dist"
  },
  "app": {
    "macOSPrivateApi": true,
    "windows": [],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 10: capabilities/default.json**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Babelay windows",
  "windows": ["main", "overlay", "onboarding"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-start-resize-dragging",
    "core:window:allow-outer-position",
    "core:window:allow-outer-size",
    "core:window:allow-current-monitor",
    "core:event:default"
  ]
}
```

- [ ] **Step 11: 빌드 확인**

```bash
cargo test -p babelay
cargo clippy -p babelay
yarn tauri build --debug --no-bundle
```

Expected: 테스트 11개 PASS, clippy 경고 0, 컴파일 성공. `yarn tauri dev`로 실행하면 온보딩 창(빈 "Babelay" 텍스트)과 트레이 아이콘이 뜨고, 트레이 메뉴의 "종료"가 동작한다.

- [ ] **Step 12: Commit**

```bash
git add src-tauri
git commit -m "feat: windows, overlay geometry, tray menu, global shortcuts, commands"
```

---

### Task 6: 프론트엔드 기반 — 타입, 설정 스토어, i18n, 테마, 창 분기

**Files:**
- Create: `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/settings.ts`, `src/lib/session.ts`, `src/lib/i18n.ts`, `src/lib/theme.ts`, `src/locales/{ko,en,ja}.json`, `src/test/{settings,theme,i18n,locales}.test.ts`
- Modify: `src/main.tsx`

**Interfaces:**
- Consumes: Task 5의 커맨드와 이벤트
- Produces:
  - `Settings` TS 타입(Rust와 동일 키), `DeepPartial<T>`
  - `mergeSettings(base: Settings, patch: DeepPartial<Settings>): Settings`
  - `useSettings()` zustand 스토어: `{ settings: Settings | null, load(): Promise<void>, update(patch): Promise<void>, subscribeBackend(): () => void }`
  - `useSession()`: `{ capturing: boolean, toggle(): void, bind(): () => void }`
  - `resolveLang(pref, navigatorLang): "ko"|"en"|"ja"`, `resolveTheme(pref, systemDark): "dark"|"light"`, `applyTheme(pref)`
  - `initI18n(lang)`; `t()` 키는 `src/locales/en.json`이 기준

- [ ] **Step 1: 실패하는 테스트 4개**

`src/test/settings.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { mergeSettings, defaultSettings } from "../lib/settings";

describe("mergeSettings", () => {
  it("applies nested patch without touching siblings", () => {
    const next = mergeSettings(defaultSettings, { overlay: { font_size: 32 } });
    expect(next.overlay.font_size).toBe(32);
    expect(next.overlay.bg_opacity).toBe(defaultSettings.overlay.bg_opacity);
    expect(next.general).toEqual(defaultSettings.general);
  });

  it("does not mutate the base", () => {
    mergeSettings(defaultSettings, { general: { theme: "dark" } });
    expect(defaultSettings.general.theme).toBe("system");
  });
});
```

`src/test/theme.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { resolveTheme } from "../lib/theme";

describe("resolveTheme", () => {
  it("follows system when pref is system", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });
  it("explicit pref wins", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });
});
```

`src/test/i18n.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { resolveLang } from "../lib/i18n";

describe("resolveLang", () => {
  it("uses navigator language for system", () => {
    expect(resolveLang("system", "ko-KR")).toBe("ko");
    expect(resolveLang("system", "ja")).toBe("ja");
    expect(resolveLang("system", "de-DE")).toBe("en");
  });
  it("explicit pref wins", () => {
    expect(resolveLang("ja", "ko-KR")).toBe("ja");
  });
});
```

`src/test/locales.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import ko from "../locales/ko.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

function keys(obj: Record<string, unknown>, prefix = ""): string[] {
  return Object.entries(obj).flatMap(([k, v]) =>
    typeof v === "object" && v !== null ? keys(v as Record<string, unknown>, `${prefix}${k}.`) : [`${prefix}${k}`],
  );
}

describe("locale files", () => {
  it("have identical key sets", () => {
    const e = keys(en).sort();
    expect(keys(ko).sort()).toEqual(e);
    expect(keys(ja).sort()).toEqual(e);
  });
  it("have no empty strings", () => {
    for (const f of [ko, en, ja]) {
      const flat = JSON.stringify(f);
      expect(flat).not.toContain('""');
    }
  });
});
```

- [ ] **Step 2: 실패 확인**

```bash
yarn test
```

Expected: 모듈을 찾지 못해 4개 파일 모두 실패.

- [ ] **Step 3: types.ts**

```ts
export type Theme = "system" | "dark" | "light";
export type UiLang = "system" | "ko" | "en" | "ja";
export type Lang = "ko" | "en" | "ja";
export type SourceLang = "auto" | Lang;
export type DisplayMode = "both" | "source" | "target";
export type Provider = "openai" | "anthropic" | "gemini" | "deepl" | "custom";

export interface Settings {
  version: number;
  general: { theme: Theme; ui_language: UiLang; onboarding_done: boolean };
  asr: { model_id: string; gpu: boolean; source_lang: SourceLang };
  translation: {
    backend: "local" | "cloud";
    local_model: string;
    cloud: { provider: Provider; model: string; base_url: string };
  };
  overlay: {
    enabled: boolean;
    monitor_id: string;
    x_ratio: number;
    y_ratio: number;
    w_ratio: number;
    display_mode: DisplayMode;
    subtitle_lang: UiLang;
    font_size: number;
    bg_opacity: number;
  };
}

export type DeepPartial<T> = { [K in keyof T]?: T[K] extends object ? DeepPartial<T[K]> : T[K] };

export interface MonitorInfo {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  primary: boolean;
}
```

- [ ] **Step 4: tauri.ts**

```ts
import { invoke } from "@tauri-apps/api/core";
import type { MonitorInfo, Settings } from "./types";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  getPlatform: () => invoke<string>("get_platform"),
  checkAudioPermission: () => invoke<"granted" | "denied" | "unknown">("check_audio_permission"),
  openPrivacySettings: () => invoke<void>("open_privacy_settings"),
  finishOnboarding: () => invoke<void>("finish_onboarding"),
  overlaySetAdjustMode: (enabled: boolean) => invoke<void>("overlay_set_adjust_mode", { enabled }),
  overlayGetMonitors: () => invoke<MonitorInfo[]>("overlay_get_monitors"),
  overlayCommitPosition: () => invoke<void>("overlay_commit_position"),
};
```

- [ ] **Step 5: settings.ts**

```ts
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import type { DeepPartial, Settings } from "./types";

export const defaultSettings: Settings = {
  version: 1,
  general: { theme: "system", ui_language: "system", onboarding_done: false },
  asr: { model_id: "small", gpu: true, source_lang: "auto" },
  translation: {
    backend: "local",
    local_model: "qwen3.5-2b",
    cloud: { provider: "openai", model: "gpt-4o-mini", base_url: "" },
  },
  overlay: {
    enabled: true,
    monitor_id: "",
    x_ratio: 0.5,
    y_ratio: 0.85,
    w_ratio: 0.6,
    display_mode: "both",
    subtitle_lang: "system",
    font_size: 24,
    bg_opacity: 0.8,
  },
};

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function merge<T>(base: T, patch: DeepPartial<T>): T {
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(patch as Record<string, unknown>)) {
    if (v === undefined) continue;
    out[k] = isObj(v) && isObj(out[k]) ? merge(out[k], v) : v;
  }
  return out as T;
}

export const mergeSettings = (base: Settings, patch: DeepPartial<Settings>): Settings => merge(base, patch);

interface SettingsStore {
  settings: Settings | null;
  load: () => Promise<void>;
  update: (patch: DeepPartial<Settings>) => Promise<void>;
  subscribeBackend: () => () => void;
}

export const useSettings = create<SettingsStore>((set, get) => ({
  settings: null,
  load: async () => set({ settings: await api.getSettings() }),
  update: async (patch) => {
    const next = mergeSettings(get().settings ?? defaultSettings, patch);
    set({ settings: next });
    await api.setSettings(next);
  },
  subscribeBackend: () => {
    const p = listen<Settings>("settings-changed", (e) => set({ settings: e.payload }));
    return () => {
      p.then((un) => un());
    };
  },
}));
```

- [ ] **Step 6: session.ts**

```ts
import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";

interface SessionStore {
  capturing: boolean;
  toggle: () => void;
  bind: () => () => void;
}

// ponytail: 2단계에서 engine-event를 받아 실제 캡처 상태로 교체한다.
export const useSession = create<SessionStore>((set, get) => ({
  capturing: false,
  toggle: () => set({ capturing: !get().capturing }),
  bind: () => {
    const p = listen("capture-toggle", () => get().toggle());
    return () => {
      p.then((un) => un());
    };
  },
}));
```

- [ ] **Step 7: theme.ts와 i18n.ts**

`src/lib/theme.ts`:

```ts
import type { Theme } from "./types";

export function resolveTheme(pref: Theme, systemDark: boolean): "dark" | "light" {
  if (pref === "system") return systemDark ? "dark" : "light";
  return pref;
}

export function applyTheme(pref: Theme) {
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const apply = () => document.documentElement.classList.toggle("dark", resolveTheme(pref, mq.matches) === "dark");
  apply();
  mq.onchange = apply;
}
```

`src/lib/i18n.ts`:

```ts
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import ko from "../locales/ko.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";
import type { Lang, UiLang } from "./types";

export function resolveLang(pref: UiLang, navigatorLang: string): Lang {
  const code = pref === "system" ? navigatorLang : pref;
  const primary = code.split(/[-_]/)[0]?.toLowerCase();
  return primary === "ko" || primary === "ja" ? primary : "en";
}

export async function initI18n(lang: Lang) {
  if (i18next.isInitialized) return i18next.changeLanguage(lang);
  return i18next.use(initReactI18next).init({
    lng: lang,
    fallbackLng: "en",
    resources: { ko: { translation: ko }, en: { translation: en }, ja: { translation: ja } },
    interpolation: { escapeValue: false },
  });
}
```

`tsconfig.json`에 `"resolveJsonModule": true`가 있는지 확인한다(없으면 추가).

- [ ] **Step 8: 로케일 파일 3개**

`src/locales/en.json`:

```json
{
  "app": { "name": "Babelay" },
  "nav": { "live": "Live", "history": "History", "settings": "Settings", "collapse": "Collapse sidebar", "expand": "Expand sidebar" },
  "status": { "idle": "Idle", "capturing": "Capturing" },
  "live": { "start": "Start", "stop": "Stop", "overlayOn": "Overlay on", "overlayOff": "Overlay off", "empty": "Press Start to begin capturing system audio.", "shortcutHint": "Shortcut: {{capture}} to start/stop, {{overlay}} to toggle overlay" },
  "history": { "empty": "No sessions yet." },
  "settings": { "general": "General", "transcription": "Transcription Model", "translation": "Translation", "overlay": "Overlay" },
  "general": {
    "theme": "Theme", "themeSystem": "System", "themeDark": "Dark", "themeLight": "Light",
    "language": "Language", "langSystem": "System default", "langKo": "Korean", "langEn": "English", "langJa": "Japanese",
    "shortcuts": "Shortcuts", "shortcutCapture": "Start / stop capture", "shortcutOverlay": "Show / hide overlay"
  },
  "models": {
    "badgeBalanced": "balanced", "badgeInstalled": "installed", "badgeInUse": "in use",
    "size": "Size", "speed": "Speed",
    "speed1": "Very slow", "speed2": "Slow", "speed3": "Medium", "speed4": "Fast", "speed5": "Very fast",
    "gpuMac": "Apple Silicon acceleration", "gpuWin": "NVIDIA GPU acceleration",
    "asrTitle": "Transcription model", "llmTitle": "Translation model",
    "download": "Download", "select": "Select", "delete": "Delete"
  },
  "translation": {
    "backend": "Backend", "local": "Local model", "cloud": "Cloud API",
    "provider": "Provider", "model": "Model", "baseUrl": "Base URL",
    "providerOpenai": "OpenAI", "providerAnthropic": "Anthropic", "providerGemini": "Google Gemini", "providerDeepl": "DeepL", "providerCustom": "Custom (OpenAI compatible)"
  },
  "overlay": {
    "monitor": "Monitor", "primary": "primary",
    "adjust": "Adjust position", "adjustOn": "Done adjusting", "adjustOff": "Adjust position",
    "adjustHint": "Drag the subtitle to move it. Drag the corner to resize.",
    "displayMode": "Display", "modeBoth": "Source + translation", "modeSource": "Source only", "modeTarget": "Translation only",
    "subtitleLang": "Subtitle language", "sourceLang": "Source language", "auto": "Auto detect",
    "fontSize": "Font size", "bgOpacity": "Background opacity",
    "sampleSource": "We buffer about two seconds of audio before running the model.",
    "sampleTarget": "The model runs after buffering about two seconds of audio."
  },
  "onboarding": {
    "stepLanguage": "Language", "stepPermission": "Permission", "stepAsr": "Transcription model", "stepLlm": "Translation model", "stepDone": "Done",
    "next": "Next", "back": "Back", "skip": "Skip", "finish": "Get started",
    "languageTitle": "Choose your language",
    "permissionTitle": "Allow system audio capture",
    "permissionDesc": "Babelay needs permission to hear what your Mac is playing. Nothing is recorded to disk.",
    "permissionCheck": "Check permission", "permissionGranted": "Permission granted", "permissionDenied": "Permission denied", "permissionUnknown": "Permission status will be checked when capture starts.",
    "openSettings": "Open System Settings",
    "asrTitle": "Choose a transcription model", "asrDesc": "The recommended model for this machine is marked balanced.",
    "llmTitle": "Choose a translation model", "llmDesc": "You can skip this and use a cloud API later.",
    "doneTitle": "All set", "doneDesc": "Open the Live page and press Start."
  }
}
```

`src/locales/ko.json` (같은 키, 한국어):

```json
{
  "app": { "name": "Babelay" },
  "nav": { "live": "라이브", "history": "히스토리", "settings": "설정", "collapse": "사이드바 접기", "expand": "사이드바 펼치기" },
  "status": { "idle": "대기", "capturing": "캡처 중" },
  "live": { "start": "시작", "stop": "정지", "overlayOn": "오버레이 켬", "overlayOff": "오버레이 끔", "empty": "시작을 누르면 시스템 오디오 캡처를 시작합니다.", "shortcutHint": "단축키: {{capture}} 시작/정지, {{overlay}} 오버레이 토글" },
  "history": { "empty": "아직 세션이 없습니다." },
  "settings": { "general": "일반", "transcription": "전사 모델", "translation": "번역", "overlay": "오버레이" },
  "general": {
    "theme": "테마", "themeSystem": "시스템", "themeDark": "다크", "themeLight": "라이트",
    "language": "언어", "langSystem": "시스템 기본", "langKo": "한국어", "langEn": "영어", "langJa": "일본어",
    "shortcuts": "단축키", "shortcutCapture": "캡처 시작 / 정지", "shortcutOverlay": "오버레이 표시 / 숨김"
  },
  "models": {
    "badgeBalanced": "balanced", "badgeInstalled": "설치됨", "badgeInUse": "사용 중",
    "size": "용량", "speed": "속도",
    "speed1": "매우 느림", "speed2": "느림", "speed3": "보통", "speed4": "빠름", "speed5": "매우 빠름",
    "gpuMac": "Apple Silicon 가속", "gpuWin": "NVIDIA GPU 가속",
    "asrTitle": "전사 모델", "llmTitle": "번역 모델",
    "download": "다운로드", "select": "선택", "delete": "삭제"
  },
  "translation": {
    "backend": "방식", "local": "로컬 모델", "cloud": "클라우드 API",
    "provider": "프로바이더", "model": "모델", "baseUrl": "Base URL",
    "providerOpenai": "OpenAI", "providerAnthropic": "Anthropic", "providerGemini": "Google Gemini", "providerDeepl": "DeepL", "providerCustom": "커스텀 (OpenAI 호환)"
  },
  "overlay": {
    "monitor": "모니터", "primary": "주",
    "adjust": "위치 조정", "adjustOn": "조정 완료", "adjustOff": "위치 조정",
    "adjustHint": "자막을 드래그해 옮기세요. 모서리를 드래그하면 크기가 바뀝니다.",
    "displayMode": "표시 모드", "modeBoth": "원문 + 번역", "modeSource": "원문만", "modeTarget": "번역만",
    "subtitleLang": "자막 언어", "sourceLang": "원어", "auto": "자동 감지",
    "fontSize": "글자 크기", "bgOpacity": "배경 투명도",
    "sampleSource": "We buffer about two seconds of audio before running the model.",
    "sampleTarget": "모델을 돌리기 전에 약 2초 분량의 오디오를 버퍼링합니다."
  },
  "onboarding": {
    "stepLanguage": "언어", "stepPermission": "권한", "stepAsr": "전사 모델", "stepLlm": "번역 모델", "stepDone": "완료",
    "next": "다음", "back": "이전", "skip": "건너뛰기", "finish": "시작하기",
    "languageTitle": "언어를 선택하세요",
    "permissionTitle": "시스템 오디오 캡처 허용",
    "permissionDesc": "Babelay가 Mac에서 재생 중인 소리를 들으려면 권한이 필요합니다. 디스크에 녹음하지 않습니다.",
    "permissionCheck": "권한 확인", "permissionGranted": "권한이 허용되었습니다", "permissionDenied": "권한이 거부되었습니다", "permissionUnknown": "권한은 캡처를 시작할 때 확인합니다.",
    "openSettings": "시스템 설정 열기",
    "asrTitle": "전사 모델을 선택하세요", "asrDesc": "이 기기에 맞는 추천 모델에 balanced 배지를 표시했습니다.",
    "llmTitle": "번역 모델을 선택하세요", "llmDesc": "건너뛰고 나중에 클라우드 API를 쓸 수도 있습니다.",
    "doneTitle": "준비 완료", "doneDesc": "라이브 페이지에서 시작을 누르세요."
  }
}
```

`src/locales/ja.json` (같은 키, 일본어):

```json
{
  "app": { "name": "Babelay" },
  "nav": { "live": "ライブ", "history": "履歴", "settings": "設定", "collapse": "サイドバーを閉じる", "expand": "サイドバーを開く" },
  "status": { "idle": "待機中", "capturing": "キャプチャ中" },
  "live": { "start": "開始", "stop": "停止", "overlayOn": "オーバーレイ オン", "overlayOff": "オーバーレイ オフ", "empty": "開始を押すとシステム音声のキャプチャが始まります。", "shortcutHint": "ショートカット: {{capture}} 開始/停止, {{overlay}} オーバーレイ切替" },
  "history": { "empty": "まだセッションがありません。" },
  "settings": { "general": "一般", "transcription": "文字起こしモデル", "translation": "翻訳", "overlay": "オーバーレイ" },
  "general": {
    "theme": "テーマ", "themeSystem": "システム", "themeDark": "ダーク", "themeLight": "ライト",
    "language": "言語", "langSystem": "システム既定", "langKo": "韓国語", "langEn": "英語", "langJa": "日本語",
    "shortcuts": "ショートカット", "shortcutCapture": "キャプチャ開始 / 停止", "shortcutOverlay": "オーバーレイ表示 / 非表示"
  },
  "models": {
    "badgeBalanced": "balanced", "badgeInstalled": "インストール済み", "badgeInUse": "使用中",
    "size": "サイズ", "speed": "速度",
    "speed1": "とても遅い", "speed2": "遅い", "speed3": "普通", "speed4": "速い", "speed5": "とても速い",
    "gpuMac": "Apple Silicon アクセラレーション", "gpuWin": "NVIDIA GPU アクセラレーション",
    "asrTitle": "文字起こしモデル", "llmTitle": "翻訳モデル",
    "download": "ダウンロード", "select": "選択", "delete": "削除"
  },
  "translation": {
    "backend": "方式", "local": "ローカルモデル", "cloud": "クラウド API",
    "provider": "プロバイダー", "model": "モデル", "baseUrl": "Base URL",
    "providerOpenai": "OpenAI", "providerAnthropic": "Anthropic", "providerGemini": "Google Gemini", "providerDeepl": "DeepL", "providerCustom": "カスタム (OpenAI 互換)"
  },
  "overlay": {
    "monitor": "モニター", "primary": "メイン",
    "adjust": "位置を調整", "adjustOn": "調整を終了", "adjustOff": "位置を調整",
    "adjustHint": "字幕をドラッグして移動します。角をドラッグするとサイズが変わります。",
    "displayMode": "表示", "modeBoth": "原文 + 翻訳", "modeSource": "原文のみ", "modeTarget": "翻訳のみ",
    "subtitleLang": "字幕の言語", "sourceLang": "音声の言語", "auto": "自動検出",
    "fontSize": "文字サイズ", "bgOpacity": "背景の不透明度",
    "sampleSource": "We buffer about two seconds of audio before running the model.",
    "sampleTarget": "モデルを実行する前に約2秒分の音声をバッファします。"
  },
  "onboarding": {
    "stepLanguage": "言語", "stepPermission": "権限", "stepAsr": "文字起こしモデル", "stepLlm": "翻訳モデル", "stepDone": "完了",
    "next": "次へ", "back": "戻る", "skip": "スキップ", "finish": "始める",
    "languageTitle": "言語を選択してください",
    "permissionTitle": "システム音声のキャプチャを許可",
    "permissionDesc": "Babelay が Mac の再生音を聞くには権限が必要です。ディスクには録音しません。",
    "permissionCheck": "権限を確認", "permissionGranted": "権限が許可されました", "permissionDenied": "権限が拒否されました", "permissionUnknown": "権限はキャプチャ開始時に確認します。",
    "openSettings": "システム設定を開く",
    "asrTitle": "文字起こしモデルを選択", "asrDesc": "この端末に合うおすすめモデルに balanced バッジを付けています。",
    "llmTitle": "翻訳モデルを選択", "llmDesc": "スキップして後でクラウド API を使うこともできます。",
    "doneTitle": "準備完了", "doneDesc": "ライブページで開始を押してください。"
  }
}
```

- [ ] **Step 9: main.tsx — 창 라벨 분기**

```tsx
import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./index.css";
import { useSettings } from "./lib/settings";
import { useSession } from "./lib/session";
import { applyTheme } from "./lib/theme";
import { initI18n, resolveLang } from "./lib/i18n";

// 페이지 컴포넌트는 Task 7~9에서 만든다. 그때까지는 임시 플레이스홀더.
const MainApp = React.lazy(() => import("./pages/MainApp"));
const OverlayWindow = React.lazy(() => import("./pages/OverlayWindow"));
const Onboarding = React.lazy(() => import("./pages/Onboarding"));

function Root() {
  const { settings, load, subscribeBackend } = useSettings();
  const bindSession = useSession((s) => s.bind);
  const [ready, setReady] = useState(false);
  const label = getCurrentWindow().label;

  useEffect(() => {
    const unsubSettings = subscribeBackend();
    const unsubSession = bindSession();
    load().then(() => setReady(true));
    return () => {
      unsubSettings();
      unsubSession();
    };
  }, []);

  useEffect(() => {
    if (!settings) return;
    applyTheme(settings.general.theme);
    initI18n(resolveLang(settings.general.ui_language, navigator.language));
  }, [settings?.general.theme, settings?.general.ui_language]);

  if (!ready || !settings) return null;
  const page = label === "overlay" ? <OverlayWindow /> : label === "onboarding" ? <Onboarding /> : <MainApp />;
  return <React.Suspense fallback={null}>{page}</React.Suspense>;
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
```

Task 7~9 전까지 컴파일이 되도록 임시 파일 세 개를 만든다. 각각 한 줄:

```tsx
// src/pages/MainApp.tsx, src/pages/OverlayWindow.tsx, src/pages/Onboarding.tsx
export default function Page() { return <div className="p-4">Babelay</div>; }
```

- [ ] **Step 10: 테스트 통과 확인**

```bash
yarn test
yarn tsc --noEmit
```

Expected: 4개 테스트 파일 모두 PASS, 타입 오류 0.

- [ ] **Step 11: Commit**

```bash
git add src
git commit -m "feat: settings store, i18n, theme, window routing"
```

---

### Task 7: 공용 컴포넌트, 접이식 사이드바, 메인 페이지, 설정 > 일반

**Files:**
- Create: `src/components/{Sidebar,Badge,PillButton,Toggle}.tsx`, `src/pages/MainApp.tsx`(교체), `src/pages/main/{Live,History}.tsx`, `src/pages/settings/General.tsx`
- Modify: 없음

**Interfaces:**
- Consumes: `useSettings`, `useSession`, `t()`
- Produces:
  - `<PillButton variant="primary"|"default"|"outline" size="sm"|"md" onClick>`; 라벨은 대문자 + 자간
  - `<Badge tone="accent"|"muted">`
  - `<Toggle checked onChange label>`
  - `<Sidebar collapsed onToggle>`; 접힘 상태는 `localStorage["babelay.sidebar"]`
  - 라우트: `#/live`, `#/history`, `#/settings/general`, `#/settings/transcription`, `#/settings/translation`, `#/settings/overlay`

- [ ] **Step 1: 공용 컴포넌트**

`src/components/PillButton.tsx`:

```tsx
import type { ButtonHTMLAttributes } from "react";

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "default" | "outline";
  size?: "sm" | "md";
};

const variants = {
  primary: "bg-accent text-accent-fg hover:brightness-110",
  default: "bg-surface text-fg hover:bg-surface-2",
  outline: "bg-transparent text-fg border border-fg-muted/60 hover:bg-surface",
};

export function PillButton({ variant = "default", size = "md", className = "", ...rest }: Props) {
  const pad = size === "sm" ? "px-3 py-1 text-[11px]" : "px-4 py-2 text-xs";
  return (
    <button
      {...rest}
      className={`rounded-full font-bold uppercase tracking-[1.4px] transition disabled:opacity-40 ${pad} ${variants[variant]} ${className}`}
    />
  );
}
```

`src/components/Badge.tsx`:

```tsx
export function Badge({ tone = "muted", children }: { tone?: "accent" | "muted"; children: React.ReactNode }) {
  const cls = tone === "accent" ? "bg-accent text-accent-fg" : "bg-surface-2 text-fg";
  return <span className={`rounded-[2px] px-1.5 py-px text-[10.5px] font-semibold capitalize ${cls}`}>{children}</span>;
}
```

`src/components/Toggle.tsx`:

```tsx
export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <label className="flex cursor-pointer items-center justify-between gap-4">
      <span>{label}</span>
      <span
        role="switch"
        aria-checked={checked}
        tabIndex={0}
        onClick={() => onChange(!checked)}
        onKeyDown={(e) => (e.key === " " || e.key === "Enter") && onChange(!checked)}
        className={`relative h-6 w-11 rounded-full transition ${checked ? "bg-accent" : "bg-surface-2"}`}
      >
        <span className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition ${checked ? "left-[22px]" : "left-0.5"}`} />
      </span>
    </label>
  );
}
```

`src/components/Sidebar.tsx`:

```tsx
import { NavLink } from "react-router";
import { useTranslation } from "react-i18next";
import { useSession } from "../lib/session";
import { useSettings } from "../lib/settings";

const item = ({ isActive }: { isActive: boolean }) =>
  `block rounded-md px-3 py-1.5 text-sm ${isActive ? "bg-surface font-bold text-fg" : "text-fg-muted hover:text-fg"}`;

export function Sidebar({ collapsed, onToggle }: { collapsed: boolean; onToggle: () => void }) {
  const { t } = useTranslation();
  const capturing = useSession((s) => s.capturing);
  const model = useSettings((s) => s.settings?.asr.model_id);
  const w = collapsed ? "w-14" : "w-52";

  return (
    <aside className={`flex ${w} shrink-0 flex-col gap-1 border-r border-surface bg-base p-3 transition-[width]`}>
      <div className="mb-2 flex items-center justify-between">
        {!collapsed && <span className="font-bold text-accent">● {t("app.name")}</span>}
        <button
          onClick={onToggle}
          aria-label={collapsed ? t("nav.expand") : t("nav.collapse")}
          className="rounded-full p-1 text-fg-muted hover:bg-surface hover:text-fg"
        >
          {collapsed ? "»" : "«"}
        </button>
      </div>
      <NavLink to="/live" className={item} title={t("nav.live")}>{collapsed ? "▶" : t("nav.live")}</NavLink>
      <NavLink to="/history" className={item} title={t("nav.history")}>{collapsed ? "≡" : t("nav.history")}</NavLink>
      <NavLink to="/settings/general" className={item} title={t("nav.settings")}>{collapsed ? "⚙" : t("nav.settings")}</NavLink>
      {!collapsed && (
        <div className="ml-3 flex flex-col gap-0.5 text-xs">
          <NavLink to="/settings/general" className={item}>{t("settings.general")}</NavLink>
          <NavLink to="/settings/transcription" className={item}>{t("settings.transcription")}</NavLink>
          <NavLink to="/settings/translation" className={item}>{t("settings.translation")}</NavLink>
          <NavLink to="/settings/overlay" className={item}>{t("settings.overlay")}</NavLink>
        </div>
      )}
      <div className="mt-auto flex items-center gap-2 rounded-md bg-base-2 p-2 text-xs text-fg-muted">
        <span className={`h-2 w-2 rounded-full ${capturing ? "bg-accent" : "bg-fg-muted"}`} />
        {!collapsed && <span>{capturing ? t("status.capturing") : t("status.idle")} · {model}</span>}
      </div>
    </aside>
  );
}
```

- [ ] **Step 2: MainApp.tsx (라우터 + 레이아웃)**

```tsx
import { useState } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router";
import { Sidebar } from "../components/Sidebar";
import Live from "./main/Live";
import History from "./main/History";
import General from "./settings/General";
import Transcription from "./settings/Transcription";
import Translation from "./settings/Translation";
import Overlay from "./settings/Overlay";

const KEY = "babelay.sidebar";

export default function MainApp() {
  const [collapsed, setCollapsed] = useState(() => {
    try { return localStorage.getItem(KEY) === "collapsed"; } catch { return false; }
  });
  const toggle = () => {
    const next = !collapsed;
    setCollapsed(next);
    try { localStorage.setItem(KEY, next ? "collapsed" : "expanded"); } catch { /* ignore */ }
  };

  return (
    <HashRouter>
      <div className="flex h-full">
        <Sidebar collapsed={collapsed} onToggle={toggle} />
        <main className="flex-1 overflow-auto p-6">
          <Routes>
            <Route path="/" element={<Navigate to="/live" replace />} />
            <Route path="/live" element={<Live />} />
            <Route path="/history" element={<History />} />
            <Route path="/settings/general" element={<General />} />
            <Route path="/settings/transcription" element={<Transcription />} />
            <Route path="/settings/translation" element={<Translation />} />
            <Route path="/settings/overlay" element={<Overlay />} />
          </Routes>
        </main>
      </div>
    </HashRouter>
  );
}
```

Task 8 전까지 컴파일되도록 `src/pages/settings/Transcription.tsx`, `Translation.tsx`, `Overlay.tsx`를 한 줄 플레이스홀더로 만든다:

```tsx
export default function Page() { return null; }
```

- [ ] **Step 3: Live.tsx와 History.tsx**

`src/pages/main/Live.tsx`:

```tsx
import { useTranslation } from "react-i18next";
import { PillButton } from "../../components/PillButton";
import { useSession } from "../../lib/session";
import { useSettings } from "../../lib/settings";

export default function Live() {
  const { t } = useTranslation();
  const { capturing, toggle } = useSession();
  const { settings, update } = useSettings();
  if (!settings) return null;
  const overlayOn = settings.overlay.enabled;

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <div className="flex gap-2">
          <PillButton variant="primary" onClick={toggle}>{capturing ? `■ ${t("live.stop")}` : `▶ ${t("live.start")}`}</PillButton>
          <PillButton onClick={() => update({ overlay: { enabled: !overlayOn } })}>
            {overlayOn ? t("live.overlayOn") : t("live.overlayOff")}
          </PillButton>
        </div>
        <span className="text-xs text-fg-muted">
          {settings.asr.source_lang.toUpperCase()} → {settings.overlay.subtitle_lang.toUpperCase()} · {settings.asr.model_id}
        </span>
      </div>
      <div className="flex-1 rounded-lg bg-base-2 p-4 text-sm text-fg-muted">
        {t("live.empty")}
        <div className="mt-2 text-xs">{t("live.shortcutHint", { capture: "⌘/Ctrl+Shift+S", overlay: "⌘/Ctrl+Shift+O" })}</div>
      </div>
    </div>
  );
}
```

`src/pages/main/History.tsx`:

```tsx
import { useTranslation } from "react-i18next";

export default function History() {
  const { t } = useTranslation();
  return <div className="rounded-lg bg-base-2 p-4 text-sm text-fg-muted">{t("history.empty")}</div>;
}
```

- [ ] **Step 4: 설정 > 일반**

`src/pages/settings/General.tsx`:

```tsx
import { useTranslation } from "react-i18next";
import { useSettings } from "../../lib/settings";
import type { Theme, UiLang } from "../../lib/types";

const select = "rounded-md bg-surface px-3 py-2 text-sm text-fg";
const label = "text-[10px] font-bold uppercase tracking-[1.2px] text-fg-muted";

export default function General() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  if (!settings) return null;

  return (
    <div className="flex max-w-md flex-col gap-6">
      <h2 className="text-2xl font-bold">{t("settings.general")}</h2>
      <div className="flex flex-col gap-1">
        <span className={label}>{t("general.theme")}</span>
        <select className={select} value={settings.general.theme} onChange={(e) => update({ general: { theme: e.target.value as Theme } })}>
          <option value="system">{t("general.themeSystem")}</option>
          <option value="dark">{t("general.themeDark")}</option>
          <option value="light">{t("general.themeLight")}</option>
        </select>
      </div>
      <div className="flex flex-col gap-1">
        <span className={label}>{t("general.language")}</span>
        <select className={select} value={settings.general.ui_language} onChange={(e) => update({ general: { ui_language: e.target.value as UiLang } })}>
          <option value="system">{t("general.langSystem")}</option>
          <option value="ko">{t("general.langKo")}</option>
          <option value="en">{t("general.langEn")}</option>
          <option value="ja">{t("general.langJa")}</option>
        </select>
      </div>
      <div className="flex flex-col gap-1">
        <span className={label}>{t("general.shortcuts")}</span>
        <div className="rounded-md bg-base-2 p-3 text-sm">
          <div className="flex justify-between"><span>{t("general.shortcutCapture")}</span><kbd>⌘/Ctrl+Shift+S</kbd></div>
          <div className="flex justify-between"><span>{t("general.shortcutOverlay")}</span><kbd>⌘/Ctrl+Shift+O</kbd></div>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 5: 수동 확인**

```bash
yarn tsc --noEmit && yarn test
yarn tauri dev
```

먼저 온보딩을 건너뛰기 위해 설정 파일에 `onboarding_done: true`를 넣는다(macOS: `~/Library/Application Support/com.babelay.app/settings.json`). 확인 항목:
- 사이드바 «/» 버튼으로 접힘·펼침, 앱 재시작 후 상태 유지
- 설정 > 일반에서 테마 변경 시 즉시 배경이 바뀜, 언어 변경 시 사이드바 라벨이 바뀜
- 트레이 "캡처 시작" 또는 ⌘⇧S를 누르면 사이드바 하단 상태 점이 초록으로 바뀜
- 라이브의 오버레이 토글 버튼으로 오버레이 창이 보였다 숨겨짐

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "feat: collapsible sidebar, live/history pages, general settings"
```

---

### Task 8: 오버레이 창 페이지와 설정 > 오버레이

**Files:**
- Create: `src/pages/OverlayWindow.tsx`(교체), `src/pages/settings/Overlay.tsx`(교체)

**Interfaces:**
- Consumes: `api.overlaySetAdjustMode`, `api.overlayGetMonitors`, `api.overlayCommitPosition`, 이벤트 `overlay-adjust-mode`, `useSettings`
- Produces: 오버레이 창은 `settings.overlay.{display_mode, font_size, bg_opacity}`를 즉시 반영. 조정 모드에서 드래그·리사이즈 후 300ms 디바운스로 `overlay_commit_position` 호출.

- [ ] **Step 1: OverlayWindow.tsx**

```tsx
import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";

export default function OverlayWindow() {
  const { t } = useTranslation();
  const settings = useSettings((s) => s.settings);
  const [adjust, setAdjust] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    const un = listen<boolean>("overlay-adjust-mode", (e) => setAdjust(e.payload));
    return () => { un.then((f) => f()); };
  }, []);

  useEffect(() => {
    if (!adjust) return;
    const win = getCurrentWindow();
    const commit = () => {
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => api.overlayCommitPosition(), 300);
    };
    const subs = [win.onMoved(commit), win.onResized(commit)];
    return () => { subs.forEach((p) => p.then((f) => f())); };
  }, [adjust]);

  if (!settings) return null;
  const { display_mode, font_size, bg_opacity } = settings.overlay;
  // ponytail: 2단계 전까지는 샘플 문장을 항상 표시한다.
  const source = t("overlay.sampleSource");
  const target = t("overlay.sampleTarget");

  return (
    <div className="flex h-full w-full items-end justify-center bg-transparent p-2">
      <div
        onMouseDown={(e) => { if (adjust && e.button === 0) getCurrentWindow().startDragging(); }}
        className={`relative max-w-full rounded-[10px] px-4 py-2 text-center text-white ${adjust ? "cursor-move ring-2 ring-accent" : ""}`}
        style={{ background: `rgba(18,18,18,${bg_opacity})`, backdropFilter: "blur(6px)" }}
      >
        {display_mode !== "target" && (
          <div style={{ fontSize: font_size * 0.6 }} className="text-fg-muted">{source}</div>
        )}
        {display_mode !== "source" && (
          <div style={{ fontSize: font_size, lineHeight: 1.3 }} className="font-bold">{target}</div>
        )}
        {adjust && (
          <>
            <div className="absolute -top-6 left-0 text-xs font-bold text-accent">{t("overlay.adjustHint")}</div>
            <div
              onMouseDown={(e) => { e.stopPropagation(); getCurrentWindow().startResizeDragging("SouthEast"); }}
              className="absolute -right-1.5 -bottom-1.5 h-3 w-3 cursor-nwse-resize rounded-[2px] bg-accent"
            />
          </>
        )}
      </div>
    </div>
  );
}
```

오버레이 창의 `<html>` 배경이 투명해야 하므로 `src/index.css`의 `body` 규칙을 아래처럼 바꾼다:

```css
body { color: var(--color-fg); font-family: var(--font-sans); font-size: 14px; }
body:not(.overlay) { background: var(--color-base); }
```

그리고 `src/main.tsx`의 `Root`에서 라벨이 `overlay`일 때 `document.body.classList.add("overlay")`를 한 번 실행한다(`useEffect` 안, `label` 의존).

- [ ] **Step 2: 설정 > 오버레이**

`src/pages/settings/Overlay.tsx`:

```tsx
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PillButton } from "../../components/PillButton";
import { api } from "../../lib/tauri";
import { useSettings } from "../../lib/settings";
import type { DisplayMode, MonitorInfo, SourceLang, UiLang } from "../../lib/types";

const label = "text-[10px] font-bold uppercase tracking-[1.2px] text-fg-muted";
const select = "rounded-md bg-surface px-3 py-2 text-sm text-fg";

export default function OverlaySettings() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [adjust, setAdjust] = useState(false);

  useEffect(() => {
    api.overlayGetMonitors().then(setMonitors);
    return () => { if (adjust) api.overlaySetAdjustMode(false); };
  }, []);

  if (!settings) return null;
  const o = settings.overlay;
  const selectedId = o.monitor_id || monitors.find((m) => m.primary)?.id || "";

  const toggleAdjust = async () => {
    const next = !adjust;
    setAdjust(next);
    await api.overlaySetAdjustMode(next);
  };

  const modes: DisplayMode[] = ["both", "source", "target"];
  const modeLabel = { both: t("overlay.modeBoth"), source: t("overlay.modeSource"), target: t("overlay.modeTarget") };

  return (
    <div className="flex max-w-xl flex-col gap-6">
      <h2 className="text-2xl font-bold">{t("settings.overlay")}</h2>

      <div className="flex flex-col gap-2">
        <span className={label}>{t("overlay.monitor")}</span>
        <div className="flex flex-wrap gap-3">
          {monitors.map((m) => (
            <button
              key={m.id}
              onClick={() => update({ overlay: { monitor_id: m.primary ? "" : m.id } })}
              className={`flex h-16 items-end justify-center rounded bg-surface px-2 pb-1 text-xs ${m.id === selectedId ? "ring-2 ring-accent text-fg" : "text-fg-muted"}`}
              style={{ width: Math.round((m.width / m.height) * 64) }}
            >
              {m.id}{m.primary ? ` (${t("overlay.primary")})` : ""}
            </button>
          ))}
        </div>
      </div>

      <div className="flex items-center justify-between">
        <span className={label}>{t("overlay.adjust")}</span>
        <PillButton variant={adjust ? "primary" : "default"} onClick={toggleAdjust}>
          {adjust ? t("overlay.adjustOn") : t("overlay.adjustOff")}
        </PillButton>
      </div>

      <div className="flex flex-col gap-2">
        <span className={label}>{t("overlay.displayMode")}</span>
        <div className="flex gap-2">
          {modes.map((m) => (
            <PillButton key={m} size="sm" variant={o.display_mode === m ? "primary" : "default"} onClick={() => update({ overlay: { display_mode: m } })}>
              {modeLabel[m]}
            </PillButton>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="flex flex-col gap-1">
          <span className={label}>{t("overlay.subtitleLang")}</span>
          <select className={select} value={o.subtitle_lang} onChange={(e) => update({ overlay: { subtitle_lang: e.target.value as UiLang } })}>
            <option value="system">{t("general.langSystem")}</option>
            <option value="ko">{t("general.langKo")}</option>
            <option value="en">{t("general.langEn")}</option>
            <option value="ja">{t("general.langJa")}</option>
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <span className={label}>{t("overlay.sourceLang")}</span>
          <select className={select} value={settings.asr.source_lang} onChange={(e) => update({ asr: { source_lang: e.target.value as SourceLang } })}>
            <option value="auto">{t("overlay.auto")}</option>
            <option value="ko">{t("general.langKo")}</option>
            <option value="en">{t("general.langEn")}</option>
            <option value="ja">{t("general.langJa")}</option>
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <span className={label}>{t("overlay.fontSize")} · {o.font_size}px</span>
          <input type="range" min={14} max={64} value={o.font_size} onChange={(e) => update({ overlay: { font_size: Number(e.target.value) } })} />
        </div>
        <div className="flex flex-col gap-1">
          <span className={label}>{t("overlay.bgOpacity")} · {Math.round(o.bg_opacity * 100)}%</span>
          <input type="range" min={0} max={100} value={Math.round(o.bg_opacity * 100)} onChange={(e) => update({ overlay: { bg_opacity: Number(e.target.value) / 100 } })} />
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: 수동 확인**

```bash
yarn tsc --noEmit
yarn tauri dev
```

확인 항목:
- 오버레이 창이 주 모니터 하단 중앙에 투명 배경으로 뜨고, 그 위의 다른 창을 클릭할 수 있다(클릭 통과)
- 설정 > 오버레이에서 글자 크기·투명도·표시 모드를 바꾸면 오버레이가 즉시 반영
- "위치 조정" 켜기 → 오버레이에 초록 테두리와 안내 문구가 뜨고 드래그로 이동, 모서리로 크기 조절. "조정 완료" 후 앱을 재시작하면 같은 위치로 복원
- 모니터가 둘이면 썸네일 클릭으로 오버레이가 해당 모니터로 이동

- [ ] **Step 4: Commit**

```bash
git add src
git commit -m "feat: overlay window with adjust mode and overlay settings page"
```

---

### Task 9: 모델 행 컴포넌트, 설정 > 전사 모델 / 번역, 온보딩

**Files:**
- Create: `src/lib/models.fixture.ts`, `src/components/ModelRow.tsx`, `src/pages/settings/Transcription.tsx`(교체), `src/pages/settings/Translation.tsx`(교체), `src/pages/Onboarding.tsx`(교체)

**Interfaces:**
- Consumes: `api.getPlatform`, `api.checkAudioPermission`, `api.openPrivacySettings`, `api.finishOnboarding`, `useSettings`
- Produces:
  - `ModelInfo { id, kind: "asr"|"llm", name, desc_key, size_bytes, speed: 1|2|3|4|5 }`, `ASR_MODELS`, `LLM_MODELS`, `formatSize(bytes)`
  - `<ModelRow model selected badges onSelect>`; `badges: { balanced?: boolean; installed?: boolean; inUse?: boolean }`

- [ ] **Step 1: 모델 픽스처**

`src/lib/models.fixture.ts`:

```ts
// ponytail: 2단계에서 엔진 레지스트리(get_models 커맨드)로 교체한다.
export interface ModelInfo {
  id: string;
  kind: "asr" | "llm";
  name: string;
  desc: string;
  size_bytes: number;
  speed: 1 | 2 | 3 | 4 | 5;
}

const MB = 1024 * 1024;
const GB = 1024 * MB;

export const ASR_MODELS: ModelInfo[] = [
  { id: "tiny", kind: "asr", name: "Whisper Tiny", desc: "fastest, low accuracy", size_bytes: 75 * MB, speed: 5 },
  { id: "base", kind: "asr", name: "Whisper Base", desc: "fast, short sentences", size_bytes: 142 * MB, speed: 4 },
  { id: "small", kind: "asr", name: "Whisper Small", desc: "balanced speed and accuracy", size_bytes: 466 * MB, speed: 3 },
  { id: "medium", kind: "asr", name: "Whisper Medium", desc: "high accuracy", size_bytes: 1.5 * GB, speed: 2 },
  { id: "large-v3-turbo", kind: "asr", name: "Whisper Large v3 Turbo", desc: "high accuracy, strong multilingual", size_bytes: 1.6 * GB, speed: 2 },
  { id: "large-v3", kind: "asr", name: "Whisper Large v3", desc: "best accuracy", size_bytes: 3.1 * GB, speed: 1 },
];

export const LLM_MODELS: ModelInfo[] = [
  { id: "gemma3-1b", kind: "llm", name: "Gemma 3 1B", desc: "fastest, simple sentences", size_bytes: 0.8 * GB, speed: 5 },
  { id: "qwen3.5-2b", kind: "llm", name: "Qwen 3.5 2B", desc: "good balance", size_bytes: 1.4 * GB, speed: 4 },
  { id: "gemma3-4b", kind: "llm", name: "Gemma 3 4B", desc: "better fluency", size_bytes: 2.5 * GB, speed: 3 },
  { id: "qwen3.5-4b", kind: "llm", name: "Qwen 3.5 4B", desc: "best quality, strong CJK", size_bytes: 2.5 * GB, speed: 3 },
];

export const BALANCED = { asr: "small", llm: "qwen3.5-2b" }; // 2단계에서 시스템 사양 기반으로 교체

export function formatSize(bytes: number): string {
  return bytes >= GB ? `${(bytes / GB).toFixed(1)} GB` : `${Math.round(bytes / MB)} MB`;
}
```

- [ ] **Step 2: ModelRow**

`src/components/ModelRow.tsx`:

```tsx
import { useTranslation } from "react-i18next";
import { Badge } from "./Badge";
import { formatSize, type ModelInfo } from "../lib/models.fixture";

interface Props {
  model: ModelInfo;
  selected: boolean;
  badges?: { balanced?: boolean; installed?: boolean; inUse?: boolean };
  onSelect: () => void;
}

export function ModelRow({ model, selected, badges = {}, onSelect }: Props) {
  const { t } = useTranslation();
  return (
    <button
      onClick={onSelect}
      className={`grid w-full grid-cols-[16px_1.4fr_1.6fr_0.6fr_0.9fr] items-center gap-3 rounded-lg bg-base-2 px-3 py-2 text-left text-sm hover:bg-surface ${selected ? "ring-1 ring-accent bg-surface" : ""}`}
    >
      <span className={`h-3 w-3 rounded-full border ${selected ? "border-accent bg-accent" : "border-fg-muted"}`} />
      <span className="flex items-center gap-2 font-bold">
        {model.name}
        {badges.balanced && <Badge tone="accent">{t("models.badgeBalanced")}</Badge>}
        {badges.inUse && <Badge>{t("models.badgeInUse")}</Badge>}
        {badges.installed && !badges.inUse && <Badge>{t("models.badgeInstalled")}</Badge>}
      </span>
      <span className="text-fg-muted">{model.desc}</span>
      <span className="text-fg-muted">{formatSize(model.size_bytes)}</span>
      <span className="flex items-center gap-2 text-fg-muted">
        <span className="relative h-1.5 w-9 rounded bg-surface-2">
          <span className="absolute inset-y-0 left-0 rounded bg-fg-muted" style={{ width: `${model.speed * 20}%` }} />
        </span>
        {t(`models.speed${model.speed}`)}
      </span>
    </button>
  );
}
```

- [ ] **Step 3: 설정 > 전사 모델**

`src/pages/settings/Transcription.tsx`:

```tsx
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ModelRow } from "../../components/ModelRow";
import { Toggle } from "../../components/Toggle";
import { ASR_MODELS, BALANCED } from "../../lib/models.fixture";
import { api } from "../../lib/tauri";
import { useSettings } from "../../lib/settings";

export default function Transcription() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const [platform, setPlatform] = useState("macos");
  useEffect(() => { api.getPlatform().then(setPlatform); }, []);
  if (!settings) return null;

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <h2 className="text-2xl font-bold">{t("settings.transcription")}</h2>
      <div className="flex flex-col gap-2">
        {ASR_MODELS.map((m) => (
          <ModelRow
            key={m.id}
            model={m}
            selected={settings.asr.model_id === m.id}
            badges={{ balanced: m.id === BALANCED.asr, inUse: settings.asr.model_id === m.id }}
            onSelect={() => update({ asr: { model_id: m.id } })}
          />
        ))}
      </div>
      <div className="max-w-md rounded-lg bg-base-2 p-4">
        <Toggle
          checked={settings.asr.gpu}
          onChange={(v) => update({ asr: { gpu: v } })}
          label={platform === "windows" ? t("models.gpuWin") : t("models.gpuMac")}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 4: 설정 > 번역**

`src/pages/settings/Translation.tsx`:

```tsx
import { useTranslation } from "react-i18next";
import { ModelRow } from "../../components/ModelRow";
import { PillButton } from "../../components/PillButton";
import { BALANCED, LLM_MODELS } from "../../lib/models.fixture";
import { useSettings } from "../../lib/settings";
import type { Provider } from "../../lib/types";

const label = "text-[10px] font-bold uppercase tracking-[1.2px] text-fg-muted";
const input = "rounded-md bg-surface px-3 py-2 text-sm text-fg";
const PROVIDERS: Provider[] = ["openai", "anthropic", "gemini", "deepl", "custom"];

export default function Translation() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  if (!settings) return null;
  const tr = settings.translation;

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <h2 className="text-2xl font-bold">{t("settings.translation")}</h2>
      <div className="flex gap-2">
        <PillButton variant={tr.backend === "local" ? "primary" : "default"} onClick={() => update({ translation: { backend: "local" } })}>{t("translation.local")}</PillButton>
        <PillButton variant={tr.backend === "cloud" ? "primary" : "default"} onClick={() => update({ translation: { backend: "cloud" } })}>{t("translation.cloud")}</PillButton>
      </div>

      {tr.backend === "local" ? (
        <div className="flex flex-col gap-2">
          {LLM_MODELS.map((m) => (
            <ModelRow
              key={m.id}
              model={m}
              selected={tr.local_model === m.id}
              badges={{ balanced: m.id === BALANCED.llm, inUse: tr.local_model === m.id }}
              onSelect={() => update({ translation: { local_model: m.id } })}
            />
          ))}
        </div>
      ) : (
        <div className="grid max-w-md gap-4">
          <div className="flex flex-col gap-1">
            <span className={label}>{t("translation.provider")}</span>
            <select className={input} value={tr.cloud.provider} onChange={(e) => update({ translation: { cloud: { provider: e.target.value as Provider } } })}>
              {PROVIDERS.map((p) => <option key={p} value={p}>{t(`translation.provider${p[0].toUpperCase()}${p.slice(1)}`)}</option>)}
            </select>
          </div>
          {tr.cloud.provider !== "deepl" && (
            <div className="flex flex-col gap-1">
              <span className={label}>{t("translation.model")}</span>
              <input className={input} value={tr.cloud.model} onChange={(e) => update({ translation: { cloud: { model: e.target.value } } })} />
            </div>
          )}
          {tr.cloud.provider === "custom" && (
            <div className="flex flex-col gap-1">
              <span className={label}>{t("translation.baseUrl")}</span>
              <input className={input} placeholder="https://api.example.com/v1" value={tr.cloud.base_url} onChange={(e) => update({ translation: { cloud: { base_url: e.target.value } } })} />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
```

API 키 입력과 연결 테스트는 3단계에서 추가한다.

- [ ] **Step 5: 온보딩**

`src/pages/Onboarding.tsx`:

```tsx
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ModelRow } from "../components/ModelRow";
import { PillButton } from "../components/PillButton";
import { ASR_MODELS, BALANCED, LLM_MODELS } from "../lib/models.fixture";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";
import type { UiLang } from "../lib/types";

type Step = "language" | "permission" | "asr" | "llm" | "done";

export default function Onboarding() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const [steps, setSteps] = useState<Step[]>(["language", "permission", "asr", "llm", "done"]);
  const [idx, setIdx] = useState(0);
  const [perm, setPerm] = useState<"granted" | "denied" | "unknown" | null>(null);

  useEffect(() => {
    api.getPlatform().then((p) => { if (p !== "macos") setSteps(["language", "asr", "llm", "done"]); });
  }, []);

  if (!settings) return null;
  const step = steps[idx];
  const next = () => setIdx((i) => Math.min(i + 1, steps.length - 1));
  const back = () => setIdx((i) => Math.max(i - 1, 0));

  const stepLabel: Record<Step, string> = {
    language: t("onboarding.stepLanguage"), permission: t("onboarding.stepPermission"),
    asr: t("onboarding.stepAsr"), llm: t("onboarding.stepLlm"), done: t("onboarding.stepDone"),
  };
  const langBtn = (v: UiLang, text: string) => (
    <PillButton key={v} variant={settings.general.ui_language === v ? "primary" : "default"} onClick={() => update({ general: { ui_language: v } })}>{text}</PillButton>
  );

  return (
    <div className="flex h-full flex-col p-6">
      <div className="mb-4 flex gap-4 text-[11px] font-bold uppercase tracking-[1.2px]">
        {steps.map((s, i) => (
          <span key={s} className={i < idx ? "text-accent" : i === idx ? "text-fg" : "text-fg-muted"}>
            {i < idx ? "✓ " : `${i + 1} `}{stepLabel[s]}
          </span>
        ))}
      </div>

      <div className="flex flex-1 flex-col gap-3 overflow-auto">
        {step === "language" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.languageTitle")}</h2>
            <div className="flex flex-wrap gap-2">
              {langBtn("system", t("general.langSystem"))}{langBtn("ko", "한국어")}{langBtn("en", "English")}{langBtn("ja", "日本語")}
            </div>
          </>
        )}
        {step === "permission" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.permissionTitle")}</h2>
            <p className="text-fg-muted">{t("onboarding.permissionDesc")}</p>
            <div className="flex gap-2">
              <PillButton variant="primary" onClick={() => api.checkAudioPermission().then(setPerm)}>{t("onboarding.permissionCheck")}</PillButton>
              <PillButton variant="outline" onClick={() => api.openPrivacySettings()}>{t("onboarding.openSettings")}</PillButton>
            </div>
            {perm && <p className="text-sm">{t(`onboarding.permission${perm[0].toUpperCase()}${perm.slice(1)}`)}</p>}
          </>
        )}
        {step === "asr" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.asrTitle")}</h2>
            <p className="text-fg-muted">{t("onboarding.asrDesc")}</p>
            {ASR_MODELS.map((m) => (
              <ModelRow key={m.id} model={m} selected={settings.asr.model_id === m.id} badges={{ balanced: m.id === BALANCED.asr }} onSelect={() => update({ asr: { model_id: m.id } })} />
            ))}
          </>
        )}
        {step === "llm" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.llmTitle")}</h2>
            <p className="text-fg-muted">{t("onboarding.llmDesc")}</p>
            {LLM_MODELS.map((m) => (
              <ModelRow key={m.id} model={m} selected={settings.translation.local_model === m.id} badges={{ balanced: m.id === BALANCED.llm }} onSelect={() => update({ translation: { local_model: m.id } })} />
            ))}
          </>
        )}
        {step === "done" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.doneTitle")}</h2>
            <p className="text-fg-muted">{t("onboarding.doneDesc")}</p>
          </>
        )}
      </div>

      <div className="mt-4 flex items-center justify-between">
        <PillButton onClick={back} disabled={idx === 0}>{t("onboarding.back")}</PillButton>
        <div className="flex gap-2">
          {step === "llm" && <PillButton variant="outline" onClick={next}>{t("onboarding.skip")}</PillButton>}
          {step === "done"
            ? <PillButton variant="primary" onClick={() => api.finishOnboarding()}>{t("onboarding.finish")}</PillButton>
            : <PillButton variant="primary" onClick={next}>{t("onboarding.next")}</PillButton>}
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 6: 수동 확인**

```bash
yarn tsc --noEmit && yarn test
rm ~/Library/Application\ Support/com.babelay.app/settings.json
yarn tauri dev
```

확인 항목:
- 첫 실행에 온보딩 창이 뜨고 5단계(언어·권한·전사·번역·완료)를 진행. 언어 버튼을 누르면 즉시 UI 언어가 바뀜
- 권한 단계 "권한 확인"을 누르면 "권한은 캡처를 시작할 때 확인합니다." 문구, "시스템 설정 열기"가 개인정보 보호 창을 연다
- 모델 행에서 Whisper Small과 Qwen 3.5 2B에 balanced 배지
- "시작하기"를 누르면 온보딩 창이 닫히고 메인 창이 뜬다. 재실행 시 온보딩 없이 메인 창
- 설정 > 전사 모델의 GPU 토글 라벨이 macOS에서 "Apple Silicon 가속"
- 설정 > 번역에서 클라우드 선택 시 프로바이더에 따라 모델/Base URL 입력란이 나타남

- [ ] **Step 7: Commit**

```bash
git add src
git commit -m "feat: model rows, transcription/translation settings, onboarding flow"
```

---

### Task 10: macOS 서명 설정과 CI

**Files:**
- Create: `src-tauri/Info.plist`, `src-tauri/entitlements.plist`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `README.md`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Produces: `yarn tauri build`가 macOS에서 서명·공증된 `.dmg`를 만든다(환경변수 있을 때). GitHub Actions에서 PR마다 테스트, 태그 푸시마다 mac/windows 빌드.

- [ ] **Step 1: Info.plist와 entitlements.plist**

`src-tauri/Info.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>NSAudioCaptureUsageDescription</key>
  <string>Babelay listens to system audio to show live subtitles. Nothing is saved to disk.</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.2</string>
</dict>
</plist>
```

`src-tauri/entitlements.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.device.audio-input</key>
  <true/>
  <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
  <true/>
</dict>
</plist>
```

- [ ] **Step 2: tauri.conf.json의 bundle 항목 보강**

`bundle`에 추가:

```json
"macOS": {
  "minimumSystemVersion": "14.2",
  "entitlements": "entitlements.plist",
  "signingIdentity": null,
  "hardenedRuntime": true
},
"windows": {
  "certificateThumbprint": null,
  "nsis": { "installMode": "currentUser" }
}
```

`signingIdentity: null`이면 Tauri가 `APPLE_SIGNING_IDENTITY` 환경변수를 읽는다. 공증은 `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`가 있을 때 자동 수행된다.

- [ ] **Step 3: 로컬 서명 빌드 확인**

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: <이름> (<TEAM_ID>)"
yarn tauri build
codesign -dv --verbose=2 "src-tauri/target/release/bundle/macos/Babelay.app" 2>&1 | grep -E "Authority|TeamIdentifier"
```

Expected: `Authority=Developer ID Application: …` 줄이 보이고, `src-tauri/target/release/bundle/dmg/Babelay_0.1.0_aarch64.dmg`가 생성된다. 인증서 이름은 `security find-identity -v -p codesigning`으로 확인한다.

- [ ] **Step 4: CI 워크플로**

`.github/workflows/ci.yml`:

```yaml
name: ci
on:
  pull_request:
  push:
    branches: [main]
jobs:
  test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: jdx/mise-action@v2
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with: { workspaces: ". -> target" }
      - run: yarn install --immutable
      - run: yarn tsc --noEmit
      - run: yarn test
      - run: cargo test --workspace
      - run: cargo clippy --workspace -- -D warnings
```

`.github/workflows/release.yml`:

```yaml
name: release
on:
  push:
    tags: ["v*"]
  workflow_dispatch:
jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-latest
            args: --target aarch64-apple-darwin
          - os: windows-latest
            args: ""
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: jdx/mise-action@v2
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: aarch64-apple-darwin }
      - uses: Swatinem/rust-cache@v2
        with: { workspaces: ". -> target" }
      - run: yarn install --immutable
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: "Babelay ${{ github.ref_name }}"
          releaseDraft: true
          args: ${{ matrix.args }}
```

2단계에서 Windows 잡에 CUDA 툴킷 설치 단계를 추가한다.

- [ ] **Step 5: README**

`README.md`:

```markdown
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
```

- [ ] **Step 6: 최종 검증**

```bash
yarn tsc --noEmit && yarn test && cargo test --workspace && cargo clippy --workspace -- -D warnings
```

Expected: 모두 통과.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Info.plist src-tauri/entitlements.plist src-tauri/tauri.conf.json .github README.md
git commit -m "ci: macos signing config, ci and release workflows"
```

---

## 완료 기준

- 첫 실행에 온보딩 → 완료 후 메인 창. 재실행은 메인 창.
- 테마 3종, UI 언어 3종 + 시스템 기본이 즉시 반영.
- 접이식 사이드바, 라이브/히스토리/설정 4페이지 이동.
- 트레이 메뉴와 전역 단축키가 캡처 플래그와 오버레이 표시를 토글.
- 오버레이 창: 투명·항상 위·클릭 통과, 조정 모드 드래그·리사이즈, 비율 저장·복원, 모니터 선택, 표시 모드·글자 크기·투명도 즉시 반영.
- 아이콘 세트 생성, macOS 서명 빌드 성공, CI 워크플로 2개.
- 2단계에서 교체할 지점은 코드에 `ponytail:` 주석으로 표시: `check_audio_permission` 스텁, `models.fixture.ts`, `session.ts`의 `capture-toggle`, 오버레이의 샘플 문장.
