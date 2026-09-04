import { describe, it, expect } from "vitest";
import { reduce, initialView } from "../lib/session";

describe("session reducer", () => {
  it("started/stopped toggle capturing and flags", () => {
    let v = reduce(initialView, { type: "started", gpu_active: false, gpu_fallback: true, model_id: "whisper-small", source_lang: "en", target_lang: "ko" });
    expect(v.capturing).toBe(true); expect(v.gpuFallback).toBe(true);
    expect(v.modelId).toBe("whisper-small"); expect(v.sourceLang).toBe("en"); expect(v.targetLang).toBe("ko");
    v = reduce(v, { type: "stopped" });
    expect(v.capturing).toBe(false); expect(v.partial).toBeNull(); expect(v.gpuFallback).toBe(false); expect(v.lagging).toBe(false);
    expect(v.targetLang).toBeNull();
  });
  it("started without a translator leaves the target null", () => {
    const v = reduce(initialView, { type: "started", gpu_active: false, gpu_fallback: false, model_id: "m", source_lang: null, target_lang: null });
    expect(v.targetLang).toBeNull();
  });
  it("partial is replaced by final and finals are capped", () => {
    let v = reduce(initialView, { type: "partial", text: "hel", lang: "en", start_ms: 0 });
    expect(v.partial?.text).toBe("hel");
    v = reduce(v, { type: "final", id: 1, text: "hello", lang: "en", start_ms: 0, end_ms: 900 });
    expect(v.partial).toBeNull(); expect(v.finals).toHaveLength(1);
    for (let i = 2; i <= 600; i++) v = reduce(v, { type: "final", id: i, text: "x", lang: "en", start_ms: i, end_ms: i + 1 });
    expect(v.finals).toHaveLength(500); expect(v.finals[0].id).toBe(101);
  });
  it("error clears the stopping lock", () => {
    const v = reduce({ ...initialView, stopping: true }, { type: "error", code: "busy_stopping", message: "" });
    expect(v.stopping).toBe(false);
  });
  it("translated attaches tgt to the matching final and ignores unknown ids", () => {
    let v = reduce(initialView, { type: "final", id: 1, text: "hello", lang: "en", start_ms: 0, end_ms: 900 });
    expect(v.lastFinalAt).toBeGreaterThan(0);
    v = reduce(v, { type: "final", id: 2, text: "world", lang: "en", start_ms: 900, end_ms: 1800 });
    v = reduce(v, { type: "translated", id: 1, text: "안녕", lang: "ko" });
    expect(v.finals[0].tgt).toBe("안녕"); expect(v.finals[1].tgt).toBeUndefined();
    const before = v.finals;
    v = reduce(v, { type: "translated", id: 99, text: "x", lang: "ko" });
    expect(v.finals).toEqual(before);
  });
  it("lagging sets and a later final clears it", () => {
    let v = reduce(initialView, { type: "lagging", queued_ms: 12000 });
    expect(v.lagging).toBe(true);
    v = reduce(v, { type: "final", id: 1, text: "a", lang: "en", start_ms: 0, end_ms: 1 });
    expect(v.lagging).toBe(false);
  });
  it("cpu_fallback turns on the badge only while capturing", () => {
    const idle = reduce(initialView, { type: "cpu_fallback", stage: "translate" });
    expect(idle.gpuFallback).toBe(false);
    let v = reduce(initialView, { type: "started", gpu_active: true, gpu_fallback: false, model_id: "m", source_lang: null, target_lang: "ko" });
    v = reduce(v, { type: "cpu_fallback", stage: "translate" });
    expect(v.gpuFallback).toBe(true);
    v = reduce(v, { type: "stopped" });
    expect(v.gpuFallback).toBe(false);
  });
});
