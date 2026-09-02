import { describe, it, expect } from "vitest";
import { ASR_MODELS, BALANCED, LLM_MODELS, formatSize } from "../lib/models.fixture";

const MB = 1024 * 1024;
const GB = 1024 * MB;

describe("formatSize", () => {
  it("formats MB below 1 GB", () => {
    expect(formatSize(75 * MB)).toBe("75 MB");
    expect(formatSize(466 * MB)).toBe("466 MB");
  });
  it("formats GB with one decimal", () => {
    expect(formatSize(1.5 * GB)).toBe("1.5 GB");
    expect(formatSize(GB)).toBe("1.0 GB");
  });
});

describe("model fixtures", () => {
  const all = [...ASR_MODELS, ...LLM_MODELS];
  it("have unique ids", () => {
    expect(new Set(all.map((m) => m.id)).size).toBe(all.length);
  });
  it("have a BALANCED pick that exists", () => {
    expect(ASR_MODELS.map((m) => m.id)).toContain(BALANCED.asr);
    expect(LLM_MODELS.map((m) => m.id)).toContain(BALANCED.llm);
  });
});
