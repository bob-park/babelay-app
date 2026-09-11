# Kraken 디자인 시스템 적용 · UI 재배치 설계

2026-09-11. 브랜치 `feature/ui-v1`. 참고: `docs/design/kraken-design.md`, 목업 `docs/design/mockups/`.

## 목표

- 앱 전체 표현층을 Kraken 디자인 시스템(퍼플 `#7132f5`, 흰 표면, 12px 라운드, 은은한 그림자)으로 바꾼다.
- 사용자가 가장 자주 하는 네 가지(시작/정지, 자막 보기, 오버레이 켜고 끄기, 원어·타겟 언어 바꾸기)를 라이브 화면 한 곳에 모은다.
- "시작" 버튼의 캡처 중 표현(어두워진 버튼, 무지개 링, 파형)은 그대로 살린다.

## 결정

| 갈림길 | 선택 | 이유 |
|---|---|---|
| 앱 뼈대 | 사이드바 제거, 상단 바(로고 · 탭 · 검색 · 시작) | 자막 뷰어라 좌우 폭이 중요. 시작/정지가 어느 페이지에서도 같은 자리 |
| 다크 테마 | 라이트 + 파생 다크, 테마 설정 유지 | 기존 다크 설정·시스템 연동을 잃지 않음. 토큰 한 벌 추가로 끝 |
| 시작 버튼 캡처 중 | 무지개 링(`aura-rainbow`) 유지, 버튼은 흰 표면, 파형은 퍼플 | 요구사항. 팔레트 밖 색은 여기 하나뿐이며 "여러 언어"로 읽힘 |
| 설정 하위 구조 | 밑줄 서브탭 | 상단 바와 결이 같고 기존 코드(탭 고정·본문 스크롤) 재사용 |
| 오버레이 자막 상자 | 어두운 반투명 유지, 조정 모드 요소만 퍼플 | 어떤 배경 위에서도 읽혀야 함 |
| 앱 아이콘 | 퍼플 그라데이션 바탕 + 흰 줄 4개 | 도크에서 브랜드가 한 번에 읽힘. 앱 안 로고와 한 벌 |
| 글꼴 | Pretendard Variable 번들(npm `pretendard`) | OFL, 한글·라틴 한 글꼴. 일본어는 시스템 글꼴 폴백 |

바꾸지 않는 것: Rust 백엔드, 설정 스키마, 라우트 경로, i18n 구조, 세션·모델·업데이트 스토어, 테마 이름(`babelay`, `babelay-light`)과 `applyTheme` 로직.

## 1. 디자인 토큰 (`src/index.css`)

daisyUI 테마 두 벌의 값을 교체한다. 이름·`color-scheme`·default 지정은 그대로.

라이트 `babelay-light`

| 토큰 | 값 |
|---|---|
| base-100 / 200 / 300 | `#ffffff` / `#f6f6f9` / `#dedee5` |
| base-content | `#101114` |
| neutral / neutral-content | `#f6f6f9` / `#101114` |
| primary / primary-content | `#7132f5` / `#ffffff` |
| secondary / secondary-content | `rgba(133,91,251,.16)` 상당의 불투명색 `#ede7fe` / `#7132f5` |
| success / success-content | `#149e61` / `#ffffff` |
| warning / warning-content | `#f59e0b` / `#101114` |
| error / error-content | `#d33a4a` / `#ffffff` |
| info / info-content | `#5741d8` / `#ffffff` |
| radius-box / field / selector | `12px` / `12px` / `6px` |
| `--color-fg-muted` | `#9497a9` |

다크 `babelay`

| 토큰 | 값 |
|---|---|
| base-100 / 200 / 300 | `#101114` / `#16171c` / `#2a2b34` |
| base-content | `#f4f4f7` |
| neutral / neutral-content | `#1c1d24` / `#f4f4f7` |
| primary / primary-content | `#855bfb` / `#ffffff` |
| secondary / secondary-content | `#2a2350` / `#b49bff` |
| success / warning / error / info | `#4cd393` / `#fbbf24` / `#f3727f` / `#9b7cff`, content 는 모두 `#101114` (error 만 `#ffffff`) |
| radius | 라이트와 동일 |
| `--color-fg-muted` | `#9497a9` |

공통

- `--depth: 0`, `--noise: 0` 유지. 카드 그림자는 유틸리티 `shadow-kr`(`rgba(0,0,0,.03) 0 4px 24px`)로 `@theme`에 `--shadow-kr` 정의.
- 글꼴: `@import "pretendard/dist/web/variable/pretendardvariable.css"` 후 `--font-sans: "Pretendard Variable", Pretendard, -apple-system, "Helvetica Neue", Arial, "Hiragino Sans", "Meiryo", sans-serif`. 본문 14px, 제목은 `font-bold tracking-tight`.
- 필(9999px) 라운드 클래스(`rounded-full` on input, `btn-circle` 제외)는 쓰지 않는다.
- `.eq-bars` 는 유지. 파형 색은 버튼에서 `text-primary` 로 지정하므로 자동으로 퍼플.
- react-toastify 변수 매핑은 그대로(테마 변수를 따라감).

## 2. 앱 뼈대

`src/components/Sidebar.tsx` 삭제 → `src/components/TopBar.tsx` 신설. `MainApp.tsx` 는 `flex-col` 로 바꾸고 접힘 상태·`localStorage` 키·`wide:` 브레이크포인트를 제거한다.

TopBar 구성(왼쪽부터): 로고(`assets/icon.svg`, 20px) · `NavLink` 탭 3개(라이브 `/live`, 히스토리 `/history`, 설정 `/settings/general`) · 빈 공간 · 검색 입력(Enter 시 `/history?q=`) · 시작/정지 버튼.

- 탭 활성: `bg-secondary text-secondary-content font-semibold`, 비활성: `text-fg-muted`. 라운드 10px.
- 설정 탭 아이콘 위의 업데이트 대기 점(`indicator`)은 설정 탭 라벨 옆으로 옮긴다.
- 시작/정지 버튼은 Sidebar 의 로직을 그대로 옮긴다: 모델 미설치 시 비활성 + 툴팁(`errors.modelMissing`), 정지 중 비활성, 캡처 중 `aura aura-rainbow` 래퍼 + `btn-neutral` + `.eq-bars text-primary`, 정지 상태 `btn-primary` + play 아이콘. 라벨은 항상 보인다(접힘 없음).
- About 대화상자(`<dialog>`, 하드웨어 정보)는 TopBar 에 두지 않고 설정 › 일반으로 옮긴다(§4).
- 높이 48px, 아래 `border-b border-base-300`.

삭제되는 i18n 키: `nav.collapse`, `nav.expand`. `nav.about` 는 설정 › 일반의 버튼 라벨로 계속 쓴다.

## 3. 라이브 (`src/pages/main/Live.tsx`)

제목 `h2` 를 없앤다. 첫 줄은 컨트롤 줄:

```
[원어 ▾] → [타겟 ▾]  ● [Whisper Small] [Qwen 3.5 2B] [지연] [CPU]        (오버레이 ⏻)
```

- 원어 select: `settings.asr.source_lang` (auto/ko/en/ja). 타겟 select: `settings.overlay.subtitle_lang` (system/ko/en/ja). 둘 다 `select select-sm` 로 설정 › 번역의 것과 같은 옵션·라벨을 쓴다. 캡처 중에도 바꿀 수 있으며 다음 세션부터 적용된다(현재 동작). 표시 모드가 "원문만"이면 타겟 select 를 숨긴다.
- 상태 점·모델 배지·지연/CPU 배지는 현재 로직 그대로. 캡처 중에는 돌고 있는 세션의 값을 보여준다.
- 오버레이: join 버튼 두 개 → `<input type="checkbox" role="switch" class="toggle toggle-primary">` 하나 + 라벨 `live.overlay`. `live.overlayOn`/`live.overlayOff` 키는 삭제.
- 자막 카드: `rounded-box border border-base-300 bg-base-100 shadow-kr`. 원문 `text-fg-muted`, 번역 `font-semibold`, 부분 인식 `text-fg-muted opacity-70`.

## 4. 설정

`Settings.tsx`: 제목 `h2` 제거. 서브탭은 `tabs tabs-border` 유지(활성 밑줄은 primary 를 따라가므로 퍼플). 본문 스크롤 구조는 그대로.

`SettingGroup`: `rounded-box border border-base-300 bg-base-100 shadow-kr divide-y divide-base-300`. `SettingRow` 는 변경 없음.

일반(`General.tsx`): 맨 아래에 섹션 `nav.about` 를 추가하고 SettingGroup 한 개에 행 하나: 라벨 `app.name` + 버전, 오른쪽에 `btn btn-sm` "`nav.about`" 버튼. 버튼이 Sidebar 에서 옮겨온 About `<dialog>` 를 연다(하드웨어 조회 로직 포함). 새 컴포넌트 `src/components/AboutDialog.tsx` 로 분리한다.

모델(`ModelPicker.tsx`, `ModelRow.tsx`): 프리셋 카드·모델 행은 `border border-base-300 bg-base-100`, 선택 시 `border-primary ring-1 ring-inset ring-primary`, hover `bg-base-200`. 배지: 추천 `badge-secondary`(퍼플 subtle), 사용 중 `badge-success badge-soft`, 설치됨 `badge-ghost`, 버거울 수 있음 `badge-warning badge-soft`. 품질 미터는 primary, 속도 미터는 fg-muted(현행).

번역·오버레이 탭: 구조 변경 없음. `SegmentedControl` 은 `join` 유지, 활성 `btn-primary`, 비활성 `btn-ghost`(현행). 오버레이 미리보기 상자의 조정 버튼·링은 primary 를 따라간다.

## 5. 히스토리 (`History.tsx`)

- 제목·검색 줄 제거(검색은 TopBar). `?q=` 처리 로직은 그대로.
- 세션 목록은 카드 하나(`rounded-box border border-base-300 bg-base-100 shadow-kr p-1`) 안에 행(`rounded-lg px-3 py-2 hover:bg-base-200`)으로. 행 왼쪽에 날짜(semibold)와 길이·조각 수(fg-muted), 오른쪽에 배지.
- 상세 화면: 뒤로 버튼 `btn-ghost`, TXT/SRT `btn`(기본), 삭제 `btn-outline btn-error`. 자막 카드는 라이브와 같은 스타일.
- 검색 결과·빈 상태는 스타일만 교체.

## 6. 온보딩 (`Onboarding.tsx`)

- `steps` 유지. 완료·현재 단계 `step-primary`(퍼플). 본문은 `max-w-2xl` 가운데 정렬 유지.
- 언어 단계 버튼: 선택 `btn-primary`, 비선택 `btn`(기본, 흰 표면 + 테두리).
- 완료 단계 체크 표시: 설치됨 `bg-success`, 미설치 `bg-error`, 대기 `bg-neutral`(현행 로직).
- 하단 이전 `btn-ghost`, 다음/시작하기 `btn-primary`.

## 7. 오버레이 창 (`OverlayWindow.tsx`)

변경 없음. 자막 상자 `rgba(18,18,18,opacity)` 유지. 조정 모드의 `ring-primary`, 핸들 `bg-primary`, 안내 배지 `bg-primary text-primary-content` 는 토큰 교체로 자동으로 퍼플이 된다.

## 8. 아이콘 (`assets/icon.svg`)

바탕 `rx=185` 사각형을 세로 그라데이션 `#8b5cf6 → #5b1ecf` 로, 줄 4개는 모두 `#ffffff` 에 투명도 1 / .85 / .6 / .35 로 바꾼다. 크기·위치는 현행 유지. `yarn icons` 로 `src-tauri/icons/*` 재생성. `assets/tray.svg` 는 단색이라 변경 없음.

## 9. 토스트·모달

- 토스트: 변수 매핑이 테마를 따라가므로 자동. 진행 바 primary.
- `ConfirmModal`, About: `modal-box` 는 `rounded-box shadow-kr`. 확인(삭제) 버튼 `btn-error`.

## 10. 검증

- `yarn test`: 기존 테스트 통과. `locales.test` 로 세 로케일 키 일치 확인(삭제 키 반영).
- `yarn build`(tsc) 통과.
- `yarn tauri dev` 로 라이트·다크 각각 라이브(정지·캡처 중)·히스토리(목록·상세·검색)·설정 4탭·온보딩 창을 눈으로 확인. 캡처 시작→정지, 오버레이 토글, 오버레이 위치 조정 모드를 실제로 돌린다.
- 도크·정보 창에서 새 아이콘 확인.

## 파일 목록

| 파일 | 작업 |
|---|---|
| `package.json` | `pretendard` 의존성 추가 |
| `src/index.css` | 토큰·글꼴·그림자 교체 |
| `src/components/Sidebar.tsx` | 삭제 |
| `src/components/TopBar.tsx` | 신설 |
| `src/components/AboutDialog.tsx` | 신설(Sidebar 에서 분리) |
| `src/pages/MainApp.tsx` | 뼈대 교체 |
| `src/pages/main/Live.tsx` | 컨트롤 줄·토글·카드 |
| `src/pages/main/History.tsx` | 제목 제거·카드 스타일 |
| `src/pages/Settings.tsx`, `settings/General.tsx` | 제목 제거·정보 행 |
| `src/components/ModelPicker.tsx`, `ModelRow.tsx`, `SettingGroup.tsx`, `ConfirmModal.tsx`, `PermissionRow.tsx` | 스타일 교체 |
| `src/pages/Onboarding.tsx` | 스타일 교체 |
| `src/locales/{ko,en,ja}.json` | `nav.collapse`, `nav.expand`, `live.overlayOn`, `live.overlayOff` 삭제 |
| `assets/icon.svg`, `src-tauri/icons/*` | 아이콘 교체·재생성 |
