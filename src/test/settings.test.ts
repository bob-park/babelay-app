import { describe, it, expect } from "vitest";
import { mergeSettings, defaultSettings } from "../lib/settings";

describe("mergeSettings", () => {
  it("applies nested patch without touching siblings", () => {
    const next = mergeSettings(defaultSettings, { overlay: { font_size: 32 } });
    expect(next.overlay.font_size).toBe(32);
    expect(next.overlay.bg_opacity).toBe(defaultSettings.overlay.bg_opacity);
    expect(next.general).toEqual(defaultSettings.general);
  });

  it("does not mutate the base", () => {
    mergeSettings(defaultSettings, { general: { theme: "dark" } });
    expect(defaultSettings.general.theme).toBe("system");
  });
});
