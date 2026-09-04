import type { Final } from "./session";
import type { DisplayMode } from "./types";

/** 오버레이 한 줄. muted 는 회색(번역 옆의 원문, 말하는 중인 부분 자막). 나머지는 굵은 흰색. */
export interface OverlayLine { text: string; muted: boolean }

/**
 * 표시 모드 → 오버레이 줄(최대 두 줄, 빈 줄은 뺀다).
 * - source: 말하는 중인 부분 자막만 회색으로. 확정 문장은 보여주지 않는다(번역도 없다).
 * - both: 번역이 붙은 세트만 — 번역을 굵게 위에, 원문을 작게 아래에.
 * - target: both 와 같은 대기, 번역 줄만.
 * both/target 에서 translated 없이 source 만 오는 건 안전 상한이 지난 경우뿐이고, 그때는 굵은 흰색 — 빈 상자보다 낫다.
 */
export function overlayLines(mode: DisplayMode, source: string, partial: string, translated = ""): OverlayLine[] {
  const lines: OverlayLine[] =
    mode === "source" ? [{ text: partial, muted: true }]
    : mode === "both" && translated ? [{ text: translated, muted: false }, { text: source, muted: true }]
    : [{ text: translated || source, muted: false }];
  return lines.filter((l) => l.text);
}

/**
 * 번역 안전 상한. 정상 흐름(1~3초)에서는 걸리지 않고, 번역이 실패했거나 큐에서 버려져 영원히 오지
 * 않는 문장에서만 원문을 풀어 준다 — 그렇지 않으면 오버레이가 옛 세트에 멈춘다.
 */
export const TRANSLATION_WAIT_MS = 15_000;

export interface OverlayPair { source: string; translated: string }

/** 번역을 더 기다릴 필요가 없는 Final: 번역이 붙었거나, 이 세션은 번역하지 않거나, 원어 == 타겟. */
function settled(f: Final, tgt: string | null): boolean {
  return f.tgt !== undefined || tgt === null || f.lang === tgt;
}

/** 마지막 final 의 번역을 아직 기다리는 중인지(안전 상한 안). */
function pending(finals: Final[], tgt: string | null, now: number, lastFinalAt: number, waitMs: number): boolean {
  const last = finals[finals.length - 1];
  return Boolean(last && !settled(last, tgt) && now - lastFinalAt < waitMs);
}

/**
 * 오버레이는 번역이 붙은 세트만 보여준다. 새 Final 이 와도 번역이 없으면 화면은 바뀌지 않고,
 * 번역이 도착하는 순간 원문+번역이 함께 교체된다(가장 최근에 번역이 붙은 Final). 안전 상한이
 * 지나도 번역이 없으면 그 원문만 보여준다.
 */
export function pairForOverlay(finals: Final[], tgt: string | null, now: number, lastFinalAt: number, waitMs = TRANSLATION_WAIT_MS): OverlayPair {
  const last = finals[finals.length - 1];
  if (!last) return { source: "", translated: "" };
  if (!settled(last, tgt) && now - lastFinalAt >= waitMs) return { source: last.text, translated: "" };
  for (let i = finals.length - 1; i >= 0; i--) {
    const f = finals[i];
    if (settled(f, tgt)) return { source: f.text, translated: f.tgt ?? "" };
  }
  return { source: "", translated: "" };
}

/** 오버레이가 100ms 타이머로 다시 그려야 하는 상태인지(번역 대기 중). */
export function awaitingTranslation(finals: Final[], tgt: string | null, now: number, lastFinalAt: number, waitMs = TRANSLATION_WAIT_MS): boolean {
  return pending(finals, tgt, now, lastFinalAt, waitMs);
}
