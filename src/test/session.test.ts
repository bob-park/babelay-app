import { describe, it, expect } from "vitest";
import { reduce, initialView } from "../lib/session";

describe("session reducer", () => {
  it("started/stopped toggle capturing and flags", () => {
    let v = reduce(initialView, { type: "started", gpu_active: false, gpu_fallback: true, model_id: "whisper-small", source_lang: "en" });
    expect(v.capturing).toBe(true); expect(v.gpuFallback).toBe(true);
    expect(v.modelId).toBe("whisper-small"); expect(v.sourceLang).toBe("en");
    v = reduce(v, { type: "stopped" });
    expect(v.capturing).toBe(false); expect(v.partial).toBeNull(); expect(v.gpuFallback).toBe(false); expect(v.lagging).toBe(false);
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
  it("lagging sets and a later final clears it", () => {
    let v = reduce(initialView, { type: "lagging", queued_ms: 12000 });
    expect(v.lagging).toBe(true);
    v = reduce(v, { type: "final", id: 1, text: "a", lang: "en", start_ms: 0, end_ms: 1 });
    expect(v.lagging).toBe(false);
  });
});
