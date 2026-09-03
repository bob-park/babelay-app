# SDD ledger — plan: docs/superpowers/plans/2026-09-03-phase3-translation.md
Spec: docs/superpowers/specs/2026-09-02-babelay-design.md (+ 2.5 spec §7.1 overlayLines)
Branch: main (standing user directive from phases 1–2.5). Shell: mise not activated → `mise exec -- cargo/yarn …`.
Ruling: user said "3단계 진행해줘"; no plan existed → plan written this session from the master spec (§11 item 3), execution mode = subagent-driven (user's choice in phases 1–2.5) — cost if wrong: none, same reviewed output.
Ruling: another session had started phase 3 in this tree at 22:57 (untracked crates/babelay-engine/src/translate/{mod,prompt}.rs + lib.rs edit). User chose "this session proceeds" and stops the other. Those ~130 lines (sync Translator trait, TranslateRequest, postprocess, system/user prompt) are fully covered by Task 1; they are removed before Task 1 dispatch so the implementer starts from HEAD — cost if wrong: ~130 lines of trivial code, reproducible from the plan.
Environment note: auto-mode safety classifier (claude-opus-5) intermittently overloaded → Bash/Agent/Skill calls fail randomly; retry.

## Pre-flight scan
| Pair / Task | Produces vs consumes | Finding |
|---|---|---|
| T1↔T2 | TranslateRequest{text,src,tgt,context}, TranslateError{Load,Request,Response,Auth}, Translator (sync), instruction(), clean() | consistent; T1 leaves `pub mod cloud;` out until T2 adds cloud.rs |
| T1↔T3 | Box<dyn Translator> is Send (trait: Send) | consistent |
| T1↔T4 | LocalLlm::load(path, gpu) -> (Self, fell_back); LocalLlm: Translator | T4 SharedLlm holds LocalLlm inside Arc<Mutex<..>> → needs LocalLlm: Send (plan notes unsafe impl fallback) |
| T2↔T4 | cloud::new(CloudConfig{provider,model,base_url,api_key}) -> Result<Cloud>; Cloud: Translator | consistent; T4 passes base_url only for custom |
| T3↔T4/T5 | EngineConfig{translator: Option<Box<dyn Translator>>, target_lang: String}; Started.target_lang: Option<String>; Translated{id,text,lang}; TranslateFailed{id,message} | T4 compiles src-tauri with temporary None/""; T5 fills from Plan |
| T3 self | Stopped ownership moves to translate thread when present; drain backstop covers both | tests expect Stopped exactly once — consistent |
| T3 self | FakeSource extended to 2 sentences (t<250) for the skip test | existing tests only need ≥1 Final — ok |
| T4↔T5 | Plan{translator,target_lang,label,warning}; plan() ignores display_mode; session applies source-only rule | consistent |
| T5 self | history::on_final removed → on_event; SessionState.segment_ids | session.rs relay loop call site updated in T5 |
| T5↔T6 | serde tag snake_case: translated / translate_failed; started.target_lang | TS union matches |
| T6↔T7 | Final{translation,translateFailed,receivedAt}; pickFinal(finals, now, holdMs); HOLD_MS; DEFAULT_MODEL/PROVIDERS; api.setApiKey/hasApiKey/deleteApiKey/testTranslation | consistent |
| T6 self | reduce("final") sets receivedAt via Date.now(); pickFinal tests pass explicit now | ok |
| T7 self | locale key test file name unknown to plan ("src/test/i18n.test.ts or locales.test.ts") | both exist: i18n.test.ts, locales.test.ts — implementer runs both |
| T1 env | ggml duplicate-symbol link risk between whisper-rs-sys and llama-cpp-sys-2 | plan Step 1 spike; ruling path (a)/(b) if it fails |
| T1 env | llama-cpp-2 API names uncertain | plan tells implementer to check the pinned version's source |
| T4 env | keyring mock builder for the test; dev builds may prompt for Keychain access | checklist item |
| T8 | docs only | — |

Ruling: T4 temporarily sets `translator: None, target_lang: String::new()` in session.rs so the workspace compiles between T3 and T5 — cost if wrong: one-line churn in T5.

## Tasks

Ruling (2026-09-03, late): the auto-mode classifier stayed down for the whole execution window — every Agent dispatch and every non-read-only Bash call was rejected ("claude-opus-5 temporarily unavailable"). Read/Write/Edit worked. Rather than stall, the lead wrote Tasks 1–7 directly from the briefs (same interfaces, same tests, TDD order collapsed into one pass per file). Consequences: (a) no per-task implementer/reviewer loop — the final whole-branch review must be run once tools return; (b) nothing has been compiled or tested yet — the first `cargo test --workspace` / `yarn test` run is the RED→GREEN check for everything at once; (c) llama-cpp-2 API names in `translate/local.rs` were taken from the plan's research notes and not re-verified against the pinned crate source. Cost if wrong: compile-fix wave, no design drift.

Deviations from briefs (intentional):
- T3 retry test uses "429 ×3 → Err(RateLimited), hits == 3" instead of "429 then 200" (httpmock 0.7 has no per-call response sequence).
- T5 `test_translation` runs as `#[tauri::command(async)]` + worker thread + `recv_timeout(20s)` instead of `spawn_blocking` (no tokio dependency in src-tauri).
- T5 adds `translator::enabled/label` helpers; `history::on_final` became `history::on_event` (handles Final + Translated).
- T6 `pairForOverlay(finals, tgt, now, lastFinalAt)` takes the target code (null = no translation expected) so source-only mode and source==target don't wait 3 s; `awaitingTranslation` drives the 100 ms timer. `SessionView.lastFinalAt` added. Extra error key `errors.timeout`.
- T2 `is_qwen3` also matches "qwen3.5" (contains "qwen3").

| Task | Status | Notes |
|---|---|---|
| 1 trait/prompt/postprocess | written | translate/{mod,prompt}.rs, 4 tests |
| 2 LocalLlm | written | translate/local.rs, Cargo features metal/cuda wired; API unverified |
| 3 cloud ×4 | written | translate/cloud.rs, 15 tests (httpmock) |
| 4 engine thread + Translated | written | engine.rs, 5 tests; e2e example updated |
| 5 keys/translator/commands/history/session | written | keys.rs, translator.rs, history segments_au + final_rows, 4 commands |
| 6 frontend | written | types/tauri/session/overlay/models, OverlayWindow/Live/History/Translation, locales ×3, tests |
| 7 docs | written | spec §4.3/§4.4/§6/§7.4/§8/§11, README, phase3 GUI checklist |
| gates | pending | blocked on classifier |
| final review | pending | blocked on classifier |

## Takeover (2026-09-04 02:30, this session)
Ruling: the other session's uncommitted implementation (33 files, +2348/-167) is kept, not reverted — it is complete through docs and all gates pass; reverting would discard finished work. The plan file on disk is THIS session's 8-task plan (it overwrote the other session's same-named plan at ~23:00); the code follows the other session's 7-task briefs (task-2..7-brief.md in this dir). The spec is the authority for the final review; the plan's interface names (pickFinal, llm.rs cache, Plan{}) are not binding — cost if wrong: plan doc drifts from code (fixed in the docs pass).
Gates (controller-run, 02:31–02:35): cargo test workspace 54+26 passed (4 ignored), vitest 38 passed, tsc clean, yarn build ok. clippy had 2 errors, fixed by the controller (mechanical: explicit_counter_loop in translate/local.rs, type_complexity in translator.rs → `type Built`), fmt applied. Ruling: controller applied those two lint fixes itself instead of dispatching — cost if wrong: none, both are behavior-neutral.
Final review: dispatched over the working tree (review-worktree.diff) — pending.
Final review (opus, worktree vs 7eec6a2): 1 Critical (C1 translate queue blocking send stalls transcription), 3 Important (I2 LLM loaded per session before capture + spec §4.3 rewritten, I3 overlay derives target from UI language ≠ backend OS locale, I4 unbounded stop drain), minors M1–M13. Verdict: With fixes. Report: final-review.md.
Committed the reviewed tree as 4 commits (engine/app/ui/docs) so the fix wave is a reviewable range.
Ruling: fix wave = C1 (try_send after Final) + I2 (process-global LLM cache keyed by path+gpu, lazy load on first translate, spec §4.3 sentence restored) + I3 (Started.target_lang) + I4 (stop discards pending translations via AtomicBool) + M1 (target mode falls back to source after the hold) + M2 (spec §7.4: drop the "그 아래 Partial" promise while a translation pair is shown) + M3 (DeepL context) + M4 (status in Request variant) + M5 (strip trailing /no_think) + M8 (context-window test) + M9 (pending() helper) + M10 (2.5 spec §7.1) + M12 (EN-US) + M13 (max_tokens for OpenAI/Gemini). Deferred: M6 (n_ctx clamp comment), M7 (async command), M11 (key change button) — cost if wrong: small UX/perf polish.
Fix wave (opus): commits 15bc260..9e2cc7f (5) — C1, I2, I3, I4, M1–M5, M8–M10, M12, M13 addressed per implementer; gates reported green (cargo 84, vitest 40). Concerns: LlmCache untested (needs GGUF/AppHandle), cache mutex held during translate (ponytail), stop discards pending translations (spec §4.4). Scoped re-review dispatched.
Re-review (opus) died: API session limit (429) for that model. Controller ran the ignored real-model test (Qwen3.5-2B, Metal): FAILED with Empty — Qwen3.5 ignores `/no_think` and spends the whole generation budget inside <think>. Ruling: controller fixed it directly (render() prefills the assistant turn with an empty <think></think> block for qwen3-family models; `/no_think` removed) — a 10-line change verified against the real model (안녕하세요, 여러분. in 1.85 s) — cost if wrong: none, covered by the ignored test which now passes. Re-review re-dispatched on sonnet over 15bc260..HEAD.
Re-review (sonnet, 15bc260..7006030): all 15 items ADDRESSED (C1, I2, I3, I4, M1–M5, M8–M10, M12, M13, Q1); no new Critical/Important. Minors (deferred): SharedLlm holds the cache mutex for a whole translation (ponytail ceiling); LlmCache has no automated test (needs GGUF/AppHandle); I4 test drives translate_loop with the flag pre-set rather than stop(); think-prefill guard assumes no open-<think> templates (none in registry); re-download over an installed path would not evict the mmap (UI cannot trigger it). Out-of-scope fixed by controller: Live header target label uses view.targetLang while capturing (1 line, tsc/vitest/build green).
Deferred from final review (not fixed, by ruling): M6 n_ctx clamp comment, M7 test_translation blocking a runtime worker, M11 key "변경" button. Cost if wrong: polish only.
Phase 3: complete (commits e420a16..HEAD; final review + scoped re-review clean). Runtime verification outstanding (user): docs/superpowers/2026-09-03-phase3-gui-checklist.md. Verified by controller with the real model: Qwen3.5-2B en→ko in 1.85 s (Metal).
