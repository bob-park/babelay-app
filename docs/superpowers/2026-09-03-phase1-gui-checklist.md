# Phase 1 manual GUI checklist (run: mise exec -- yarn tauri dev)
1. First launch: onboarding window appears (5 steps on macOS). Language buttons switch UI text immediately.
2. Permission step: "권한 확인" shows "권한은 캡처를 시작할 때 확인합니다." and "시스템 설정 열기" opens Privacy settings.
3. Model steps: Whisper Small / Qwen 3.5 2B carry the balanced badge; descriptions are in the UI language. "시작하기" closes onboarding, opens main window; relaunch skips onboarding.
4. Sidebar « » collapse persists across relaunch; collapsed width does not clip the glyphs.
5. Settings > General: theme change repaints instantly (light/dark/system); language change updates sidebar AND tray menu labels.
6. Tray "캡처 시작" or ⌘⇧S turns the sidebar status dot green; ⌘⇧O / tray toggles the overlay window.
7. Overlay: transparent, bottom-center of the primary monitor, click-through (clicks reach windows beneath). Font size / opacity / display mode sliders update it live.
8. Settings > Overlay > 위치 조정: overlay gets an accent ring + hint chip; drag moves it, SE corner resizes; "조정 완료" restores click-through; relaunch restores the same position.
9. Critical-path regression: turn adjust mode ON, then close the main window (red button). Click the bottom-center of the screen — clicks must pass through (adjust mode must have exited). Reopen from tray: the adjust pill must read OFF.
10. Race regression: drag the opacity slider continuously while pressing ⌘⇧O — the overlay must stay hidden after the drag ends (tray change is not clobbered).
11. Error surface: make settings.json read-only (chmod 444 "~/Library/Application Support/com.babelay.app/settings.json"), change a setting — a dismissible error bar appears and the control snaps back. Restore permissions afterwards.
12. Multi-monitor (if available): clicking a monitor thumbnail moves the overlay to that monitor.
