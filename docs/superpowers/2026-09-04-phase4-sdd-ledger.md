# SDD ledger — plan: docs/superpowers/plans/2026-09-04-phase4-passthrough-device.md
Spec: docs/superpowers/specs/2026-09-04-phase4-passthrough-device-design.md (+ master spec 2026-09-02)
Branch: main (standing user directive from phases 1–3, recorded in the phase-4 spec). Shell: `mise exec -- cargo/yarn …`.
Base before Task 1: e907965.

## Pre-flight scan
| Pair / Task | Produces vs consumes | Finding |
|---|---|---|
| T1↔T2 | both edit engine.rs — T1 transcribe_loop + LangVote, T2 chunker_loop + tests | disjoint regions; sequential dispatch — consistent |
| T1↔T4 | T4 adds EngineEvent::CpuFallback to the enum T1 leaves alone | consistent |
| T2↔T8 | T8 frames carry stream.rate/channels (may change on reconnect); T2 resampler follows format | consistent |
| T3↔T4 | translator.rs — T3 adds target(); T4 swaps SharedLlm{..} literal for SharedLlm::new | disjoint hunks — consistent |
| T4 self | LlmCache{slot, app: Option<AppHandle>} derives Default+Clone (Option<AppHandle> is Default); existing test uses LlmCache::default() | consistent; lock() must move from .0 to .slot |
| T4↔T5 | commands.rs keeps crate::llm::cache(&app) | consistent |
| T3 self | test: Settings::default display_mode "both" → enabled; target() ignores model install | consistent |
| T6 self | uses common.cancel (exists in ko; locales test enforces en/ja) + new translation.changeKey ×3 | consistent |
| T7 self | stop before queue exists → close_aggregate direct; after open fails → queue exists, listener NULL → dispatch_sync path | consistent; ARC: void* + CFBridgingRetain/Release per brief |
| T8 self | wasapi 0.24 names (get_id, Handle) verified in Step 1 before coding | brief carries the check |
| T9 | docs only | — |
| Rubric | every specified test asserts behaviour; no duplicated logic blocks mandated | clean |

## Tasks
Task 1+2 (batched, haiku): implemented — commits 05bd395 (LangVote), 3d84248 (resampler swap); 62 passed, clippy/fmt clean. Review (sonnet) dispatched over e907965..3d84248.
Task 1: minor (deferred): Resampler::format is pub (pub(crate) would do); Option::insert would remove the `expect("just set")`; Partial.lang still raw per-segment detection (not shown in UI today).
Task 1: complete (commits e907965..05bd395, review clean)
Task 2: complete (commits 05bd395..3d84248, review clean)
Base before Task 3+5: 3d84248.
Task 3+5 (batched, haiku): implemented — commits 71ed974 (target()), 07a0a5b (async test_translation); workspace tests green, clippy clean. Implementer concern: precheck/build still gate on enabled(), so en→en still needs the model/key.
Ruling: keep precheck/build on enabled() — spec §3.2 says precheck stays as is (misconfiguration is reported before start); the plan's GUI checklist line "로컬 모델이 설치돼 있지 않아도 시작된다" contradicts the spec and is corrected in Task 9's dispatch — cost if wrong: one extra start_failed banner for a user with en→en and no model, fixable by one condition swap.
Review (sonnet) dispatched over 3d84248..07a0a5b.
Task 3: minor (deferred): history tgt_label falls back to raw subtitle_lang ("system") when target() is None (pre-existing path via display_mode=source); translator::label still keys off enabled() so en→en records local:<model>.
Task 3: complete (commits 3d84248..71ed974, review clean)
Task 5: complete (commits 71ed974..07a0a5b, review clean)
Base before Task 4: 07a0a5b.
Task 4 (sonnet): implemented — commit 42ffcd4; gates green (cargo 27+62, vitest 41, tsc, clippy). Review (sonnet) dispatched over 07a0a5b..42ffcd4.
Task 4: minor (deferred): notified flag set before the app.is_some check (Default cache burns it on a no-op); connection test + live session can each emit once (reducer idempotent); no Rust test on the emit path (needs Tauri mock runtime).
Task 4: complete (commits 07a0a5b..42ffcd4, review clean)
Base before Task 6: 42ffcd4.
Task 6 (haiku): implemented — commit 5cd9f28; tsc/vitest 41/build green. Review (haiku) dispatched over 42ffcd4..5cd9f28.
Task 6: minor (deferred): 변경/삭제 are adjacent identical ghost buttons and delete has no confirm (misclick drops the key; recoverable); key.trim() predicate duplicated; saveKey promise chain lacks an unmount guard (pre-existing).
Task 6: complete (commits 42ffcd4..5cd9f28, review clean)
Base before Task 7: 5cd9f28.
Task 7 (opus): implementer dispatched over base 5cd9f28.
Task 7 (opus): implemented — commit 4a5b63d; 62 tests, ignored capture test passed, headless device-flip harness showed two rebuilds with frames continuing. Deviation: listener guard `if (cur && [now isEqualToString:cur]) return;` instead of the brief's `!cur ||` early return.
Ruling: accept the deviation — the brief's guard would make a failed rebuild permanent (out_uid is nil right after a failure), contradicting the spec's "retry on the next notification"; the implementer's form retries — cost if wrong: none, it is strictly the spec's behaviour.
Review (opus) dispatched over 5cd9f28..4a5b63d.
Task 7 review (opus): Approved with 2 Important — I1 residual window where a HAL notification dispatched after RemovePropertyListenerBlock returns could run on freed h (plan-mandated stop sequence); I2 AddPropertyListenerBlock OSStatus discarded (registration failure would silently disable the feature). Minors: start-time missed switch between open_aggregate and listener registration (ms window); NSLog prints "(null) -> X" on a retry; kDefaultOutputAddr + function reorder beyond the brief (harmless).
Ruling: I1 parked as a documented residual — the stop sequence is the spec's (remove listener → dispatch_sync teardown); blocks enqueued before removal are FIFO-ordered ahead of the teardown, and Apple's HAL removal is documented as synchronous for its own state; no cheap airtight fix exists (deferring free only moves the boundary) — cost if wrong: a crash on stop coinciding with a device switch within the same millisecond; mitigation later = listener captures a heap-allocated alive flag instead of h.
Task 7: minor (deferred): start-time missed switch window; "(null) -> X" log on retry; kDefaultOutputAddr/reorder beyond brief.
Task 7: fix round 1/5 dispatched for I2 only.
Task 7: fix round 1/5 (I2 fixed per implementer — commit 1f1967a; verification: clang -Wall -Wextra clean, cargo build warning-free, 62 tests, device-flip harness re-run ok). Scoped re-review (sonnet) dispatched over 4a5b63d..1f1967a.
Task 7: fix round 1/5 (1 addressed, 0 open — I2 listener status checked; commits 4a5b63d..1f1967a)
Task 7: complete (commits 5cd9f28..1f1967a, review clean, I1 parked with ruling)
Base before Task 8: 1f1967a.
Task 8 (sonnet): implementer dispatched over base 1f1967a.
Task 8 (sonnet): implemented — commit bea8542; workspace tests green; isolated x86_64-pc-windows-msvc check clean (run with rustup toolchain directly — mise's Homebrew rust has no windows std; wasapi 0.24 names matched the brief). Concerns noted: idle wake now does one extra read per second; reopen retries forever at 1 Hz with a log line each (spec: retry every 1 s until stop); no hardware verification.
Review (sonnet) dispatched over 1f1967a..bea8542.
Task 8: minor (deferred): unbounded 1 Hz reopen log when no usable render endpoint (no post-start error channel to the engine) — wants a log-once counter or a ponytail: line; failed-handle spin ponytail note understates the new re-enumerate/reopen cost; device switch undetected while an app pinned to the old endpoint keeps events flowing (spec-conformant; fix = wall-clock compare once/s).
Task 8: complete (commits 1f1967a..bea8542, review clean)
Base before Task 9: bea8542.
Task 9 (sonnet): docs implementer dispatched over base bea8542 with corrections (checklist en→en line, retry item, §4.1 Windows sentence, §4.3 once-per-SharedLlm wording).
Task 9 (sonnet): implemented — commit 520c5f5; docs only, grep checks clean. Review (haiku) dispatched over bea8542..520c5f5.
Final gates (controller, at 520c5f5): fmt ok, clippy clean, cargo test 27+62 passed (4 ignored), tsc ok, vitest 41/41, yarn build ok.
Task 9: minor (deferred): §4.1 "1초마다 비교" reads as unconditional (id compare happens only on timeout wakes; live switches are caught by the read-error branch); §4.3 nested parentheses; phase-4 spec line 5 still future tense.
Task 9: complete (commits bea8542..520c5f5, review clean)
All tasks complete. Final whole-branch review (opus) dispatched over e907965..520c5f5.
Final review (opus, e907965..520c5f5): With fixes. I1 tap.m parked UAF has a cheap airtight fix (__block BOOL alive shared by listener + teardown block on the same serial queue); I2 session.rs tgt_label falls back to raw subtitle_lang ("system") → use translator::resolve_tgt, and translator::label should key off target().is_none(). Minors M1–M11 (timestamp drift after a device outage, silent permanent rebuild failure, start-time window, 1 Hz log, spin-comment cost, timeout-only detection, expect("just set"), 변경/삭제 adjacency, missing docs/superpowers phase-4 sdd-ledger + stale README checklist pointer, LlmCache↔AppHandle Arc cycle, short target-language interjections now translate). Report: final-review.md.
Ruling (reverses the Task 7 I1 parking): the parking premise "no cheap airtight fix" was wrong — fix now in the wave — cost if wrong: none, the guard is 3 lines and only narrows behaviour.
Ruling: fix wave = I1 + I2 (+ label guard and test) + docs (README checklist pointer → phase 4, §4.1 "1초마다" → "타임아웃으로 깰 때마다", Windows lines in the GUI checklist). M1/M2/M4 and the en→en precheck friction go to the spec §11 backlog line. Other minors deferred as ledgered. The docs/superpowers phase-4 sdd-ledger is written by the controller after the wave (it is this ledger's export).
Fix wave (opus): commits 6ffb8fc (I1 __block alive + teardown block; I2 resolve_tgt fallback + label via target()?), bb25066 (docs). cargo 89 passed, clippy clean, clang clean, ignored capture test passed, device-flip harness + 40× switch/stop race stress green. Scoped re-review (sonnet) dispatched over 520c5f5..bb25066.
Final gates (controller, at bb25066): fmt ok, clippy clean, cargo test 27+62 passed, tsc ok, vitest 41/41, build ok, tree clean.
Fix-wave re-review (opus, 520c5f5..bb25066): I1, I2a, I2b, docs ×4 all ADDRESSED; no new breakage. Carry-over: docs/superpowers phase-4 sdd-ledger (written by the controller below).
Phase 4: complete (commits 05bd395..bb25066; final review + scoped re-review clean). Runtime verification outstanding (user): docs/superpowers/2026-09-04-phase4-gui-checklist.md (macOS 10 items + Windows 3 items).
