import { describe, it, expect } from "vitest";
import { rowAction, formatSize } from "../lib/models";
import type { ModelStatus } from "../lib/types";

const base: ModelStatus = {
  info: { id: "small", kind: "asr", name: "Whisper Small", desc_key: "models.desc.small", size_bytes: 466 * 1024 * 1024, speed: 3, url: "https://x", filename: "s.bin", sha256: null },
  installed: false, in_use: false, balanced: true, download: null,
};

describe("rowAction", () => {
  it("not installed → download", () => expect(rowAction(base)).toBe("download"));
  it("downloading → cancel", () => expect(rowAction({ ...base, download: { received: 1, total: 2 } })).toBe("cancel"));
  it("installed → select", () => expect(rowAction({ ...base, installed: true })).toBe("select"));
  it("in use → delete (disabled by UI)", () => expect(rowAction({ ...base, installed: true, in_use: true })).toBe("delete"));
  it("in use but not installed → download", () => expect(rowAction({ ...base, in_use: true })).toBe("download"));
  it("downloading wins over installed flag", () => expect(rowAction({ ...base, installed: true, download: { received: 1, total: 2 } })).toBe("cancel"));
});

describe("formatSize", () => {
  it("formats MB and GB", () => {
    expect(formatSize(75 * 1024 * 1024)).toBe("75 MB");
    expect(formatSize(1.5 * 1024 ** 3)).toBe("1.5 GB");
  });
});
