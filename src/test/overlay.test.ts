import { describe, it, expect } from "vitest";
import { awaitingTranslation, overlayLines, pairForOverlay, TRANSLATION_WAIT_MS } from "../lib/overlay";
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
  it("keeps the previous translated set until the new translation arrives", () => {
    const finals = [f(1, "a", "ㄱ"), f(2, "b")];
    expect(pairForOverlay(finals, "ko", 9_500, 9_000)).toEqual({ source: "a", translated: "ㄱ" });
    expect(pairForOverlay(finals, "ko", 14_000, 9_000)).toEqual({ source: "a", translated: "ㄱ" }); // 5초가 지나도 원문만 먼저 보이지 않는다
    expect(awaitingTranslation(finals, "ko", 14_000, 9_000)).toBe(true);
  });
  it("skips untranslated finals and shows the latest translated set", () => {
    const finals = [f(1, "a", "ㄱ"), f(2, "b"), f(3, "c")];
    expect(pairForOverlay(finals, "ko", 11_500, 11_000)).toEqual({ source: "a", translated: "ㄱ" });
    finals[1] = f(2, "b", "ㄴ"); // 2번 번역이 3번보다 먼저 도착
    expect(pairForOverlay(finals, "ko", 11_500, 11_000)).toEqual({ source: "b", translated: "ㄴ" });
  });
  it("shows nothing before the first translation arrives", () => {
    const finals = [f(1, "a")];
    expect(pairForOverlay(finals, "ko", 9_100, 9_000)).toEqual({ source: "", translated: "" });
    expect(awaitingTranslation(finals, "ko", 9_100, 9_000)).toBe(true);
  });
  it("falls back to the source alone only after the safety cap (failed or dropped translation)", () => {
    const finals = [f(1, "a", "ㄱ"), f(2, "b")];
    expect(pairForOverlay(finals, "ko", 9_000 + TRANSLATION_WAIT_MS, 9_000)).toEqual({ source: "b", translated: "" });
    expect(awaitingTranslation(finals, "ko", 9_000 + TRANSLATION_WAIT_MS, 9_000)).toBe(false);
    expect(TRANSLATION_WAIT_MS).toBe(15_000);
  });
  it("does not wait when no translation will come", () => {
    const finals = [f(1, "a"), f(2, "b")];
    expect(pairForOverlay(finals, null, 9_100, 9_000)).toEqual({ source: "b", translated: "" }); // 원문만 모드
    expect(pairForOverlay([f(1, "a"), f(2, "b", undefined, "ko")], "ko", 9_100, 9_000)).toEqual({ source: "b", translated: "" }); // 원어 == 타겟
    expect(awaitingTranslation([f(1, "a", "ㄱ"), f(2, "b", undefined, "ko")], "ko", 9_100, 9_000)).toBe(false);
  });
});

describe("overlayLines", () => {
  it("target mode shows the source once the safety cap expires", () => {
    const finals = [f(1, "a", "\u3131"), f(2, "b")];
    const late = pairForOverlay(finals, "ko", 9_000 + TRANSLATION_WAIT_MS, 9_000);
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
