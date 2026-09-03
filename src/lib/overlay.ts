import type { DisplayMode } from "./types";

export interface OverlayLines { primary: string; secondary: string }

/** 표시 모드 → 오버레이 두 줄. 번역이 있으면 both 는 번역을 크게, 원문을 작게 보여준다. */
export function overlayLines(mode: DisplayMode, source: string, partial: string, translated = ""): OverlayLines {
  if (mode === "target") return { primary: translated, secondary: "" };
  if (mode === "both" && translated) return { primary: translated, secondary: source };
  return { primary: source, secondary: partial };
}
