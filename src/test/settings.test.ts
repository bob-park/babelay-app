import { describe, it, expect, vi } from "vitest";
import { mergeSettings, defaultSettings, useSettings } from "../lib/settings";
import { api } from "../lib/tauri";
import type { Settings } from "../lib/types";

vi.mock("../lib/tauri", () => ({
  api: { getSettings: vi.fn(), setSettings: vi.fn() },
}));

const h = vi.hoisted(() => ({
  listeners: {} as Record<string, (e: { payload: Settings }) => void>,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, cb: (e: { payload: Settings }) => void) => {
    h.listeners[name] = cb;
    return Promise.resolve(() => {});
  },
}));

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

describe("useSettings.update", () => {
  it("rolls back the optimistic state when the backend rejects", async () => {
    useSettings.setState({ settings: defaultSettings });
    vi.mocked(api.setSettings).mockRejectedValueOnce(new Error("disk full"));

    await expect(useSettings.getState().update({ overlay: { font_size: 40 } })).rejects.toThrow("disk full");
    expect(useSettings.getState().settings).toBe(defaultSettings);
  });

  it("ignores a settings-changed echo while an update is in flight", async () => {
    useSettings.setState({ settings: defaultSettings });
    const unsub = useSettings.getState().subscribeBackend();

    let finish!: () => void;
    vi.mocked(api.setSettings).mockReturnValueOnce(new Promise<void>((r) => (finish = r)));
    const inFlight = useSettings.getState().update({ overlay: { font_size: 40 } });

    const stale = mergeSettings(defaultSettings, { overlay: { font_size: 10 } });
    h.listeners["settings-changed"]!({ payload: stale });
    expect(useSettings.getState().settings?.overlay.font_size).toBe(40);

    finish();
    await inFlight;

    h.listeners["settings-changed"]!({ payload: stale });
    expect(useSettings.getState().settings?.overlay.font_size).toBe(10);
    unsub();
  });
});
