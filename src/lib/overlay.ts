import type { DisplayMode } from "./types";

export interface OverlayLines { primary: string; secondary: string }

/** 표시 모드 → 오버레이 두 줄. 3단계에서 translated 를 채우면 both 도 번역 줄을 얻는다. */
export function overlayLines(mode: DisplayMode, source: string, partial: string, translated = ""): OverlayLines {
  if (mode === "target") return { primary: translated, secondary: "" };
  return { primary: source, secondary: partial };
}
