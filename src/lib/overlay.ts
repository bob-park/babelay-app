import type { Final } from "./session";
import type { DisplayMode } from "./types";

export interface OverlayLines { primary: string; secondary: string }

/**
 * 표시 모드 → 오버레이 두 줄(항상 두 줄까지). 번역이 있으면 both 는 번역을 크게, 원문을 작게
 * 보여준다. target 은 번역이 없으면(대기 만료·실패) 원문으로 내려간다 — 빈 상자보다 낫다.
 */
export function overlayLines(mode: DisplayMode, source: string, partial: string, translated = ""): OverlayLines {
  if (mode === "target") return { primary: translated || source, secondary: "" };
  if (mode === "both" && translated) return { primary: translated, secondary: source };
  return { primary: source, secondary: partial };
}

/** 번역을 이만큼 기다린다. 넘으면 원문만 보여준다. */
export const TRANSLATION_WAIT_MS = 3000;

export interface OverlayPair { source: string; translated: string }

/**
 * 마지막 final 의 번역을 아직 기다리는 중인지. `tgt` 가 null 이거나(번역 안 함) 원어가 타겟과
 * 같으면 번역이 오지 않으므로 기다리지 않는다.
 */
function pending(finals: Final[], tgt: string | null, now: number, lastFinalAt: number, waitMs: number): boolean {
  const last = finals[finals.length - 1];
  return Boolean(last && !last.tgt && tgt !== null && last.lang !== tgt && now - lastFinalAt < waitMs);
}

/**
 * 원문과 번역은 한 세트로 바뀐다. 마지막 final 의 번역이 아직 없으면 잠시(waitMs) 직전 세트를
 * 유지하고, 그래도 안 오면 원문만 보여준다.
 */
export function pairForOverlay(finals: Final[], tgt: string | null, now: number, lastFinalAt: number, waitMs = TRANSLATION_WAIT_MS): OverlayPair {
  const last = finals[finals.length - 1];
  if (!last) return { source: "", translated: "" };
  if (last.tgt) return { source: last.text, translated: last.tgt };
  const prev = finals[finals.length - 2];
  if (prev && pending(finals, tgt, now, lastFinalAt, waitMs)) return { source: prev.text, translated: prev.tgt ?? "" };
  return { source: last.text, translated: "" };
}

/** 오버레이가 100ms 타이머로 다시 그려야 하는 상태인지(번역 대기 중). */
export function awaitingTranslation(finals: Final[], tgt: string | null, now: number, lastFinalAt: number, waitMs = TRANSLATION_WAIT_MS): boolean {
  return pending(finals, tgt, now, lastFinalAt, waitMs);
}
