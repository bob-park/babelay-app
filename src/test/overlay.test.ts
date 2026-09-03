import { describe, it, expect } from "vitest";
import { awaitingTranslation, overlayLines, pairForOverlay } from "../lib/overlay";
import type { Final } from "../lib/session";

const f = (id: number, text: string, tgt?: string, lang = "en"): Final =>
  ({ id, text, lang, start_ms: id * 1000, end_ms: id * 1000 + 900, ...(tgt ? { tgt } : {}) });

describe("pairForOverlay", () => {
  it("is empty without finals", () => {
    expect(pairForOverlay([], "ko", 0, 0)).toEqual({ source: "", translated: "" });
  });
  it("shows the set when the last final is translated", () => {
    expect(pairForOverlay([f(1, "a", "ㄱ"), f(2, "b", "ㄴ")], "ko", 10_000, 9_000)).toEqual({ source: "b", translated: "ㄴ" });
  });
  it("keeps the previous set while the translation is pending", () => {
    const finals = [f(1, "a", "ㄱ"), f(2, "b")];
    expect(pairForOverlay(finals, "ko", 9_500, 9_000)).toEqual({ source: "a", translated: "ㄱ" });
    expect(awaitingTranslation(finals, "ko", 9_500, 9_000)).toBe(true);
  });
  it("falls back to the source alone after the wait", () => {
    const finals = [f(1, "a", "ㄱ"), f(2, "b")];
    expect(pairForOverlay(finals, "ko", 12_000, 9_000)).toEqual({ source: "b", translated: "" });
    expect(awaitingTranslation(finals, "ko", 12_000, 9_000)).toBe(false);
  });
  it("does not wait when no translation will come", () => {
    const finals = [f(1, "a"), f(2, "b")];
    expect(pairForOverlay(finals, null, 9_100, 9_000)).toEqual({ source: "b", translated: "" }); // 원문만 모드
    expect(pairForOverlay([f(1, "a"), f(2, "b", undefined, "ko")], "ko", 9_100, 9_000)).toEqual({ source: "b", translated: "" }); // 원어 == 타겟
    expect(pairForOverlay([f(1, "a")], "ko", 9_100, 9_000)).toEqual({ source: "a", translated: "" }); // 직전 세트 없음
  });
});

describe("overlayLines", () => {
  it("target mode shows the source once the wait expires", () => {
    const finals = [f(1, "a", "\u3131"), f(2, "b")];
    const late = pairForOverlay(finals, "ko", 12_000, 9_000);
    expect(overlayLines("target", late.source, "", late.translated)).toEqual({ primary: "b", secondary: "" });
  });
  it("puts the translation first in both mode and only the translation in target mode", () => {
    expect(overlayLines("both", "src", "", "tgt")).toEqual({ primary: "tgt", secondary: "src" });
    expect(overlayLines("both", "src", "par", "")).toEqual({ primary: "src", secondary: "par" });
    expect(overlayLines("target", "src", "par", "tgt")).toEqual({ primary: "tgt", secondary: "" });
    expect(overlayLines("target", "src", "par")).toEqual({ primary: "src", secondary: "" }); // 번역이 없으면 원문으로 내려간다
    expect(overlayLines("target", "", "par")).toEqual({ primary: "", secondary: "" });
    expect(overlayLines("source", "src", "par", "tgt")).toEqual({ primary: "src", secondary: "par" });
  });
});
