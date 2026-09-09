import { describe, it, expect } from "vitest";
import { matchPreset, presetState } from "../lib/presets";
import type { ModelStatus, PresetStatus } from "../lib/types";

const presets: PresetStatus[] = [
  { id: "fast", asr: "base", llm: "gemma3-1b", total_bytes: 1, heavy: false },
  { id: "balanced", asr: "small", llm: "qwen3.5-2b", total_bytes: 2, heavy: false },
  { id: "quality", asr: "qwen3-asr-0.6b", llm: "hy-mt2-1.8b", total_bytes: 3, heavy: true },
];
const model = (id: string, installed = false, in_use = false): ModelStatus => ({
  info: { id, kind: "asr", name: id, desc_key: "models.desc.small", size_bytes: 1, total_bytes: 1, speed: 3, quality: 3, url: "https://x", filename: id, sha256: null, mmproj: null },
  installed, in_use, balanced: false, fit: "good", download: null,
});

describe("matchPreset", () => {
  it("finds the preset whose both models match", () => {
    expect(matchPreset(presets, "small", "qwen3.5-2b")).toBe("balanced");
    expect(matchPreset(presets, "qwen3-asr-0.6b", "hy-mt2-1.8b")).toBe("quality");
  });
  it("is null when only one side matches or nothing matches", () => {
    expect(matchPreset(presets, "small", "gemma3-1b")).toBeNull();
    expect(matchPreset(presets, "", "")).toBeNull();
  });
});

describe("presetState", () => {
  it("is installed / in use only when both models are", () => {
    const models = [model("small", true, true), model("qwen3.5-2b", true, false), model("base")];
    expect(presetState(presets[1], models)).toEqual({ installed: true, inUse: false });
    expect(presetState(presets[0], models)).toEqual({ installed: false, inUse: false });
    expect(presetState(presets[1], [model("small", true, true), model("qwen3.5-2b", true, true)])).toEqual({ installed: true, inUse: true });
  });
});
