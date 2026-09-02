# SDD ledger — plan: docs/superpowers/plans/2026-09-02-phase1-app-shell.md
Spec: docs/superpowers/specs/2026-09-02-babelay-design.md
Branch: main (user explicitly requested "현재 브랜치에서 작업"; no worktree)
Shell note: mise not activated in tool shell → all JS commands as `mise exec -- yarn …`

## Pre-flight scan
| Pair / Task | Produces vs consumes | Finding |
|---|---|---|
| T1↔T5 | tauri.conf.json scaffold → rewritten in T5 | consistent |
| T1↔T6 | src/main.tsx placeholder → replaced in T6; index.css → body rule edited in T8 | consistent |
| T3↔T5 | SettingsState::{get,set}, Overlay: PartialEq | used identically in commands/overlay/tray |
| T4↔T5 | i18n::{resolve,tray_labels} | consistent |
| T5↔T6 | command names/args (settings, enabled), events settings-changed/capture-toggle/overlay-adjust-mode, MonitorInfo fields | consistent |
| T6↔T7/8/9 | useSettings.update, useSession, locale keys (overlay.primary, onboarding.permission{Granted,Denied,Unknown}, translation.provider*, models.speed1..5) | all keys present in 3 locales |
| T6/T7 placeholders ↔ T7/8/9 replacements | MainApp/OverlayWindow/Onboarding, settings pages | consistent |
| T5↔T10 | tauri.conf bundle block extended | consistent |
| T1 self | yarn test with no test files | plan already notes --passWithNoTests |
| T5 self | setup() `?` on String errors | plan notes map_err wrapper |
| T8 self | Overlay settings page cleanup effect reads stale `adjust` (deps []) → never disables adjust mode on unmount | DEFECT — see ruling |
| T7/T8/T9 self | verification steps are manual GUI checks (yarn tauri dev) | subagents verify with tsc+vitest+`tauri build --debug --no-bundle`; GUI checks deferred to user |

Ruling: T8 cleanup effect must be `useEffect(() => () => { api.overlaySetAdjustMode(false); }, [])` (unconditional, separate effect) — stale closure in plan text would leave overlay click-through disabled after leaving the page — cost if wrong: one redundant IPC call on unmount.
Ruling: manual GUI verification steps in T7–T9 are replaced by `yarn tsc --noEmit && yarn test && yarn tauri build --debug --no-bundle` for subagents; GUI checklist handed to the user at finish — cost if wrong: runtime UI bugs surface only at user's first run.

## Tasks
Task 1: minor (deferred): src-tauri/Cargo.toml authors=["you"] placeholder; public/tauri.svg dead asset; .vscode/extensions.json template leftover; yarn YN0004 esbuild build-scripts notice (benign)
Task 1: complete (commits 86848fa..49262a4, review clean)
Task 2: minor (deferred): gen-icons.mjs uses URL.pathname (non-portable) and first-occurrence string replace; android/ios icon dirs (~168K) committed — controller's dispatch allowed them under a size threshold
Task 2: complete (commits 49262a4..fc49c41, review clean)
Task 3: review (843f7d0) — 2 Important (plan-mandated test weakness: vacuous roundtrip, defaults unasserted); minors: silent non-NotFound read error, no fsync, set() not atomic across callers, lock().unwrap(); dead-code warnings until T5
Task 3: minor (deferred): settings.rs Err(_) read branch prints nothing; no fsync before rename; set() lock not held across save+store
Task 3: fix round 1/5 (2 addressed, 0 open; commits 843f7d0..95f16a6)
Task 3: complete (commits fc49c41..95f16a6, review clean)
Task 4: minor (deferred): i18n tests don't cover `_` separator / uppercase / empty; dead unwrap_or("") at i18n.rs:15
Task 4: complete (commits 95f16a6..93b63a0, review clean)
Task 5: review (ab1f39b) — 2 Important: (1) no reposition on adjust-mode exit, (2) ADJUST_MODE stored before window guard + never cleared by toggle_overlay. Fix round 1 dispatched (resume implementer). Ruling: also adopt minor "commit_position reads inner_size" in the same round — one-line, keeps set_size/commit roundtrip symmetric — cost if wrong: none observable while overlay has no decorations.
Task 5: minor (deferred): tray "open" bypasses onboarding; settings-changed echoes to caller (T6 store must tolerate echo); read-modify-write races in commit_position/toggle_overlay; #[allow(dead_code)] on whole TrayLabels; cargo fmt dirty repo-wide; icon_as_template unconditional
Task 5: fix round 1/5 (3 addressed, 0 open; commits ab1f39b..5b751ac)
Task 5: minor (deferred): set_settings during adjust mode can still hide/show at dragged geometry; lifecycle paths untested (need AppHandle); toggle_overlay emits overlay-adjust-mode:false on every toggle (idempotent)
Task 5: complete (commits 93b63a0..5b751ac, review clean)
Task 6: review (315c1cc) — 3 Important (plan-mandated): applyTheme MQL listener leak; optimistic update without rollback; settings-changed echo race. Fix round 1 dispatched (resume implementer). Ruling: echo suppression via in-flight counter (ignore settings-changed while pending>0) — external changes during a write are dropped until the next echo — cost if wrong: a tray toggle landing mid-slider-drag is not reflected until next settings event.
Task 6: minor (deferred): applyTheme/initI18n untested under node env; crude '""' empty-string check and no placeholder parity in locales test; no .catch on listen promises; initI18n double-init guard is post-resolve; Root subscribes to whole store
Task 6: fix round 1/5 (3 addressed, 0 open; commits 315c1cc..ec1f9d3)
Task 6: minor (deferred): whole-snapshot rollback can revert a concurrent update; echo suppression drops other-window changes while pending (self-heals on next echo/load); theme.ts module state has no reset hook
Task 6: complete (commits 5b751ac..ec1f9d3, review clean)
Task 7: review (a5d22af) — 2 Important (plan-mandated): sidebar parent+child settings links both active; Toggle span[role=switch] has no accessible name and Space scrolls. Fix round 1 dispatched. Ruling: parent settings link styled active only when collapsed && in /settings; Toggle → native checkbox role=switch with peer styling; also add catch-all route — cost if wrong: minor visual/keyboard differences.
Task 7: minor (deferred): Badge/Toggle unused until T8/T9; PillButton className override order-dependent, no type="button"; update() rejections uncaught at call sites (Live/General); Live start/stop only flips frontend state (ponytail, phase 2); collapsed w-14 may squeeze glyphs (needs GUI check)
Task 7: fix round 1/5 (3 addressed, 0 open; commits a5d22af..73756b3)
Task 7: minor (deferred): parent settings NavLink still emits aria-current on /settings/general (use Link); Toggle knob `transition` doesn't animate `left`
Task 7: complete (commits ec1f9d3..73756b3, review clean)
Task 8: review (56d58d9) — 4 Important (plan-mandated): source line text-fg-muted theme-dependent on dark pill; adjust hint green text over desktop; body.overlay class added in effect → opaque startup flash; monitor labels overflow thumbnails. Fix round 1 dispatched. Ruling: fixed colors on overlay pill (text-white/70, accent chip), body class at module scope, truncate labels — cost if wrong: cosmetic only.
Task 8: minor (deferred): pending commit dropped if adjust turned off within 300ms of drag; settings-page adjust pill can go stale (tray turns adjust off; event is emit_to overlay only); toggleAdjust has no catch; bg_opacity 0 → no backdrop (text-shadow suggested); every slider tick writes settings.json
Task 8: fix round 1/5 (5 addressed, 0 open; commits 56d58d9..9c77072)
Task 8: complete (commits 73756b3..9c77072, review clean)
Task 9: review (79bfb21) — spec ❌ text-accent on stepper; 2 Important (plan-mandated): ModelRow no aria-pressed; model desc untranslated English literals. Fix round 1 dispatched. Ruling: desc → desc_key with models.desc.* keys in 3 locales (spec requires user-facing 특징 in UI language) — cost if wrong: 30 locale strings to maintain.
Task 9: minor (deferred): onboarding idx may transiently exceed shortened step list; formatSize 1023.5MB boundary; fixture kind/speed unasserted; cloud form inputs lack associated labels (app-wide pattern, also General/Overlay); 7 unused models.* locale keys
Task 9: fix round 1/5 (3 addressed, 0 open; commits 79bfb21..5a8f0c7)
Task 9: complete (commits 9c77072..5a8f0c7, review clean)
Task 10: Ruling: no codesigning identity on this machine (security find-identity → 0) — brief Step 3 (signed build + codesign -dv) replaced by unsigned `yarn tauri build` + plist/dmg verification; signed build must be verified by the user with their Developer ID — cost if wrong: signing/notarization config only proven in CI or on the user's first signed build.
Task 10: dispatched (implementer sonnet, base 5a8f0c7)
Task 10: review (bd8c55b) — 2 Important: release.yml lacks permissions: contents: write; entitlement allow-unsigned-executable-memory (plan-mandated). Fix round 1 dispatched. Ruling: drop allow-unsigned-executable-memory now (no JIT in app; add allow-jit in phase 2 only if runtime demands) — cost if wrong: first notarized build crashes at launch and the key must be re-added.
Task 10: minor (deferred): workflow_dispatch from main creates a "main" draft release; aarch64 target installed on windows leg; mise yarn install on windows-latest unexercised; ci.yml no concurrency cancel; audio-input entitlement inert without sandbox (mic would need NSMicrophoneUsageDescription); README lacks prerequisites (mise + Rust)
Task 10: fix round 1/5 (2 addressed, 0 open; commits bd8c55b..821b330)
Task 10: complete (commits 5a8f0c7..821b330, review clean)
Final review: dispatched (opus) over 86848fa..821b330
User directive (2026-09-03): builds are local only; CI removed. Commit after 821b330 deletes .github/, updates README + spec §9.2/§11. Ruling: final-review findings about ci.yml/release.yml are moot and will be parked as such.
Final review (opus, 86848fa..821b330): 1 Critical (adjust mode latches when main window hides), 5 Important (#2 echo suppression clobbers backend writes, #3 stale adjust pill, #4 silent IPC failures, #5 tray labels static — plan gap, #6 CI removed → README must list all 4 verification commands). Verdict: With fixes.
Ruling: fix wave = Critical #1 + Important #2–#6 + cheap minors (cargo fmt once, drop 3 unused capabilities, set_settings adjust-mode guard via exit_adjust_mode). #2 fixed at the root: backend `set_settings` becomes patch-merge (engine will be a second writer in phase 2) and the store merges echoes under the in-flight patch — cost if wrong: ~30 extra lines vs the 5-line frontend-only fix. #5 fixed now (spec §3.2 says tray labels follow UI language) — cost if wrong: ~20 lines of tray relabel code.
Ruling: all other final-review minors and the deferred list stay deferred (documented in this ledger); T10 items moot after CI removal.
Final fix wave: commits d04f7b3..f266d5d (7): fmt, exit_adjust_mode owner, tray relabel, patch_settings + echo merge, error surface, README gates, cleanup catch. Gates: cargo 15, vitest 18, clippy/fmt/tsc/build clean. Scoped re-review dispatched (sonnet).
Note: leftover `git stash@{0}` (WIP on d04f7b3) — contents verified already committed (HEAD newer for Overlay.tsx); `git stash drop` denied by permission classifier — left for the user.
Final fix wave re-review: all 6 findings + 3 minors ADDRESSED; no Critical/Important breakage. Gates re-run independently: cargo 15+1, vitest 18, tsc clean.
Parked (final): toggle_overlay show-before-hide redundancy — Ruling: harmless same-turn, leave; enter path returns without event when overlay window missing — Ruling: window is never destroyed in phase 1, leave; set_ignore_cursor_events failure short-circuits emit/reposition — Ruling: flag already cleared, C1 not reintroduced, leave; settings.test.ts stale test name — Ruling: cosmetic.
