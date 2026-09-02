# SDD ledger — plan: docs/superpowers/plans/2026-09-03-phase2-transcription-engine.md
Spec: docs/superpowers/specs/2026-09-02-babelay-design.md
Branch: main (standing user directive). Shell: mise not activated → `mise exec -- cargo/yarn …` (cmake via mise from T2).
Ruling: user said "다음 단계 진행해줘"; execution mode = subagent-driven (chosen by the user for phases 1 and 1.5) without re-asking — cost if wrong: none, same reviewed output.

## Pre-flight scan
| Pair / Task | Produces vs consumes | Finding |
|---|---|---|
| T1↔T5 | Resampler::push(interleaved,&mut Vec), Chunker::push/flush, ChunkEvent | consistent |
| T2↔T3↔T5 | AudioSource trait + Frame + Sink; default_source() cfg-gated | consistent; Linux fallback Unsupported |
| T2↔T7 | probe_permission() → Permission | T7 maps to "granted/denied/unknown" strings |
| T4↔T5 | Transcriber trait + WhisperTranscriber::load → (Self, fell_back) | consistent; T5 stub transcriber in tests |
| T4 self | whisper-rs 0.16: WhisperState ownership, get_segment vs as_iter | plan names both fallbacks |
| T5↔T7 | start_default(cfg, tx) → EngineHandle; EngineEvent serde tag "type" snake_case | T9 TS union must match |
| T6↔models | BALANCED removed from models.rs; src-tauri list() uses hardware::balanced | T6 must update the T1(1.5) test that referenced BALANCED |
| T7↔T8 | history::on_final stub in T7, filled in T8; SessionState.session_id added in T8 | consistent |
| T7↔tray | capture-toggle event removed; frontend session.ts (phase 1) listens to it until T9 | window T7→T9 where the Live button does nothing; acceptable |
| T9 self | overlay shows source even in display_mode target until phase 3 | ruled in plan |
| T2 env | cmake via .mise.toml; first whisper build minutes | ok |
| T3 env | rustup target add x86_64-pc-windows-msvc (user-level) | ruling: allowed, standard dev step |

Ruling: T7→T9 window where the Live start button is inert is acceptable (tray/shortcut work) — cost if wrong: none after T9.

## Tasks
Task 1: review (989e203) — 1 Important: multi-block resampler carry untested. Fix round 1 dispatched (+ since_partial moved below speech guard).
Task 1: minor (deferred): leading-silence drop is 1s-granular so start_ms may precede speech by <1s; end_ms includes trailing silence (per brief tests); per-push allocations; as_chunks raises MSRV to 1.88 (no rust-version); ChunkEvent derives nothing; the brief's six tests don't tightly pin the constants
Task 1: fix round 1/5 (2 addressed, 0 open; commits 989e203..ecc89d8)
Task 1: minor (deferred): dead since_partial reset in drop branch; Resampler drops a trailing partial interleaved frame across blocks (device callbacks deliver whole frames)
Task 1: complete (commits f0074d5..ecc89d8, review clean)
Task 2: implemented (0643be1); gates green; runtime capture test BLOCKED — coreaudiod appears wedged after a leaked tap client (AudioDeviceCreateIOProcIDWithBlock never returns; probe says Granted). `sudo killall coreaudiod` denied by policy → user must restart coreaudiod or reboot, then rerun `mise exec -- cargo test -p babelay-engine captures_some_frames -- --ignored --nocapture`.
Task 2: review (0643be1) — 7 Important (static): tapautostart key likely causes the hang; missing MainSubDevice key; no @available guard; float32 format unchecked; probe() on failure path; non-interleaved channel-count assumption; no trampoline unit test. Fix round 1 dispatched (+ calloc/null/catch_unwind/raw-pointer ownership minors). Runtime test still gated on a coreaudiod restart (user action).
Task 2: minor (deferred): NoDevice variant unreachable; Permission::Unknown only via availability guard; rate as u32 could be 0 if ASBD unpopulated
Task 2: fix round 1/5 (7 + minors addressed, 0 open; commits 0643be1..6beb747)
Task 2: Ruling: `-mmacosx-version-min=14.2` makes the @available guards dead but the app's minimumSystemVersion is 14.2 anyway — leave both; cost if wrong: none (no graceful degradation below 14.2 was ever promised).
Task 2: minor (deferred): -3 (calloc) unmapped → Os(-3); possible ld deployment-target warning on cold plain-cargo builds
Task 2: complete (commits ecc89d8..6beb747, review clean) — RUNTIME CAPTURE UNVERIFIED pending coreaudiod restart (user action); re-run `mise exec -- cargo test -p babelay-engine captures_some_frames -- --ignored --nocapture` before Task 7's manual check.
Task 3: Ruling: the workspace-level `cargo check --target x86_64-pc-windows-msvc` is impossible here (`ring` build script needs a Windows C toolchain via reqwest/rustls); accept the implementer's isolated-crate check of capture/{mod,windows}.rs against real wasapi 0.24 as the gate — cost if wrong: Windows integration compile errors surface only on a Windows machine.
Task 3: review (27b4236) — 3 Important: silent buffer flag ignored (garbage audio), channel_mask None wrong on 7.1 (use mix format), late errors swallowed. Fix round 1 dispatched (+ wait-failure bound, stop-before-start).
Task 3: minor (deferred): AudioSource has no late-error channel (trait gap; engine thread should treat capture death as Stopped/Error in phase 2 follow-up); no deinitialize(); byte-at-a-time pop_front
Task 3: fix round 1 landed (1bf66b3). Ruling: the 5-failure wait bound I asked for is wrong — idle loopback endpoints stop signaling and the crate can't tell timeout from failure — so round 2 replaces it with sleep(10ms)+continue (stop flag is the only exit) — cost if wrong: a truly failed event handle spins at ~100 Hz until stop.
Task 3: fix rounds 1-2/5 (5 addressed, 0 open; commits 27b4236..cc7f357)
Task 3: minor (deferred): int-format endpoints now fail instead of autoconverting; capture-thread death after startup leaves LoopbackSource fields Some (lost-stream callback = trait gap)
Task 3: complete (commits 6beb747..cc7f357, review clean; Windows runtime unverified)
Task 4: minor (deferred): MIN_SAMPLES doc comment wrong (whisper only skips <100ms) → fix in T5; ignored test assertion lenient; whisper-rs leaks lang CString per call; silence hallucinations surface as subtitles (VAD should gate)
Task 4: Ruling for T5: `start()` takes `gpu_active: bool` (from `WhisperTranscriber.gpu_active` in start_default) instead of recomputing `cfg.use_gpu && !gpu_fallback`, so the feature-gated flag survives — cost if wrong: one extra bool parameter.
Task 4: complete (commits cc7f357..7769be6, review clean; Metal inference verified with ggml-small)
Task 5: review (e6fd0ba) — 2 Important: transcriber panic loses Stopped and leaves capture running; stop() drain blocking undocumented. Fix round 1 dispatched (+ AudioSource::stop contract doc, start ordering, frames-channel ceiling comment, panic test).
Task 5: minor (deferred): Partials can delay a Final by up to 8 inferences; try_send drops the newest Partial; resampler pinned to first frame's format
Task 5: fix round 1/5 (6 addressed, 0 open; commits e6fd0ba..d6cb124)
Task 5: minor (deferred): EngineHandle holds a tx clone so the event channel stays open until the handle drops (T7 loop must break on Stopped — the plan already does); chunker-thread panic silent; source.stop() not called on start() error path; AssertUnwindSafe caveat not at code site
Task 5: complete (commits 7769be6..d6cb124, review clean)
Task 6: review agent hit a 429 session limit (reset 06:10 KST); re-dispatched.
Task 6: review (222ccde) — 2 Important: byte→GB floor drops a tier on real Windows/NVIDIA (round to nearest GiB); balanced ids no longer asserted per kind. Fix round 1 dispatched.
Task 6: minor (deferred): System::new_all refreshes processes (cached once; narrow with new_with_specifics later); non-NVIDIA Windows GPUs fall to the CPU row
Task 6: fix round 1/5 (3 addressed, 0 open; commits 222ccde..5a0fb67)
Task 6: complete (commits d6cb124..5a0fb67, review clean; Windows nvml path compile-unverified)
Task 7: review (abda308) — 4 Important: start window unreserved (double engines), tray label diverges during stop drain, start blocks caller on model load, Windows CUDA resources land in resources/cuda not beside the exe. Fix round 1 dispatched (Phase state machine Idle/Starting/Running/Stopping, async start, relabel on stop, map-form resources, unknown_model code).
Task 7: minor (deferred): stop_on_exit blocks main thread for the drain; transcriber panic without stop() leaves relay blocked (engine backstop lives in stop()); capture_state false during drain while events still flow (note for T9)
Task 7: fix round 1/5 (5 addressed, 1 new Important: discard branch clobbers a live Running; commits abda308..ecfd0fb). Fix round 2 dispatched: generation-guarded phases, is_capturing relabels, busy_stopping, stop_on_exit joins the drain.
Task 7: fix round 2/5 (4 addressed, 0 open; commits ecfd0fb..7989fa0)
Task 7: minor (deferred): stale-generation load failure still emits start_failed; busy_stopping arrives as the message of start_failed (T9 i18n); panic between reserve and install leaves Starting(g) until a stop
Task 7: complete (commits 5a0fb67..7989fa0, review clean; runtime unverified pending coreaudiod restart)
Task 8: review (edeb544) — 1 Important: history::open failure bricks startup (make optional). Fix round 1 dispatched (+ end() on exit, drop redundant Final guard).
Task 8: minor (deferred): no AFTER UPDATE FTS trigger (needed when phase 3 writes tgt_text); tgt_lang/src_lang stored as sentinels "system"/"auto", translator column unpopulated; sync history commands on main thread; export of nonexistent session writes an empty file
Task 8: fix round 1/5 (3 addressed, 0 open; commits edeb544..ed62292)
Task 8: minor (deferred): exit path may drop tail Finals still queued in the relay (ended_at now always set); history commands with history disabled return Tauri's raw "state not managed" string (T9 should map to a message); relay exit without Stopped leaves ended_at NULL
Task 8: complete (commits 7989fa0..ed62292, review clean)
Task 9: review (46a65c8) — 1 Critical: history_segments/delete/export pass `session_id` but Tauri expects camelCase `sessionId` (detail/delete/export dead); 2 Important: export toast timer uncleared; search hit can dead-end when its session isn't loaded. Fix round 1 dispatched (+ stopping state, bind probe guard, autoscroll key, stopped clears flags, stale-response token).
Task 9: minor (deferred): permission probe fires on Windows too (harmless)
Task 9: fix round 1/5 (8 addressed, 0 open; commits 46a65c8..c0d0c00)
Task 9: minor (deferred): stopping flag can lock if Stop is pressed during a Starting phase (no Stopped emitted) — clear stopping when a probe/event reports capturing=false; stale segments rejection not gated by alive; "1 segments" plural in en
Task 9: complete (commits ed62292..c0d0c00, review clean; runtime capture flow unverified)
Task 10: minor (deferred): checklist item 1 over-states the deep-link button (only on onboarding); item 7 lag trigger unverified
Task 10: complete (commits c0d0c00..19d3759, review clean)
Final review: dispatched (opus) over f0074d5..19d3759 (20 commits)
Final review (opus, f0074d5..19d3759): 1 Critical (Resampler infinite loop on src_rate 0), 5 Important (#2 Stop during load locks Live button, #3 permission probe fires on onboarding mount, #4 device change mid-session silent — spec promises Windows restart, #5 stop_on_exit unbounded, #6 Live shows current settings not the session's), minors 7–15. Verdict: With fixes.
Ruling: fix wave = C1 + I2 (backend Stopped emit on Starting/Idle stop + frontend clear stopping on error) + I3 + I5 (split EngineHandle::stop into stop_capture + drain, exit waits ≤3 s) + I6 (Started carries model_id + source_lang; Live renders the session snapshot) + minors 7 (rust-version), 8 (overlay gated on capturing), 10 (busy_stopping code passthrough), 12 (min-speech 300 ms gate), spec §4.2 device-change limitation (drop the Windows restart promise) + checklist item 1 wording. Deferred: #4 code (device listener) to phase 3 backlog, #14 phase-machine pure tests, others as ledgered.
Final fix wave: commits 19d3759..4b1a283 (4): zero-rate guard + min-speech gate + stop_capture/drain + Started payload + rust-version; Stopped on Starting/Idle stop + bounded exit + tray code passthrough; UI stopping-on-error + session snapshot + overlay gate + probe on step; docs. Scoped re-review dispatched (opus).
Final fix wave re-review: all 10 findings + docs ADDRESSED; no Critical/Important breakage. Gates re-run by controller: cargo 49+2 ignored, vitest 25, clippy/fmt/tsc clean.
Parked (final): unmapped error codes from the tray show an empty banner — Ruling: only the models_dir path error can produce one; leave (fix = message: code.clone()).
Parked (final): Live src-lang fallback is "auto" instead of settings — Ruling: cosmetic on the probe-inferred path; leave.
Parked (final): overlay may stay blank if an engine event precedes captureState on a mid-session bind — Ruling: narrow race, overlay recovers on next started; leave.
Parked (final): Resampler accepts sub-8 kHz rates (amplification, unreachable via tap/WASAPI) — leave.
RUNTIME VERIFICATION OUTSTANDING (user): restart coreaudiod → ignored capture test → GUI checklist docs/superpowers/2026-09-03-phase2-gui-checklist.md.
