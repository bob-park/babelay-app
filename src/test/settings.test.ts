import { describe, it, expect, vi, beforeEach } from "vitest";
import { toast } from "react-toastify";
import { mergeSettings, defaultSettings, useSettings } from "../lib/settings";
import { api } from "../lib/tauri";
import type { Settings } from "../lib/types";

vi.mock("react-toastify", () => ({ toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn(), info: vi.fn(), update: vi.fn(), dismiss: vi.fn(), isActive: vi.fn(() => false) }) }));

vi.mock("../lib/tauri", () => ({
  api: { getSettings: vi.fn(), patchSettings: vi.fn() },
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

beforeEach(() => vi.clearAllMocks());

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

  it("defaults auto_update to on", () => {
    expect(defaultSettings.general.auto_update).toBe(true);
  });
});

describe("useSettings.update", () => {
  it("rolls back and surfaces the error when the backend rejects", async () => {
    useSettings.setState({ settings: defaultSettings });
    vi.mocked(api.patchSettings).mockRejectedValueOnce(new Error("disk full"));

    await useSettings.getState().update({ overlay: { font_size: 40 } });
    expect(useSettings.getState().settings).toBe(defaultSettings);
    expect(toast.error).toHaveBeenCalledWith("disk full", { toastId: "disk full" });
  });

  it("sends only the patch, not the whole document", async () => {
    useSettings.setState({ settings: defaultSettings });
    vi.mocked(api.patchSettings).mockResolvedValueOnce(undefined);

    await useSettings.getState().update({ overlay: { font_size: 40 } });
    expect(api.patchSettings).toHaveBeenLastCalledWith({ overlay: { font_size: 40 } });
  });

  it("ignores a settings-changed echo while an update is in flight", async () => {
    useSettings.setState({ settings: defaultSettings });
    const unsub = useSettings.getState().subscribeBackend();

    let finish!: () => void;
    vi.mocked(api.patchSettings).mockReturnValueOnce(new Promise<void>((r) => (finish = r)));
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

  it("keeps a backend-originated field from an echo that arrives mid-write", async () => {
    useSettings.setState({ settings: defaultSettings });
    const unsub = useSettings.getState().subscribeBackend();

    let finish!: () => void;
    vi.mocked(api.patchSettings).mockReturnValueOnce(new Promise<void>((r) => (finish = r)));
    const inFlight = useSettings.getState().update({ overlay: { font_size: 40 } });

    // 트레이가 오버레이를 끈 결과가 쓰기 도중에 도착한다.
    const fromTray = mergeSettings(defaultSettings, { overlay: { enabled: false } });
    h.listeners["settings-changed"]!({ payload: fromTray });
    expect(useSettings.getState().settings?.overlay.enabled).toBe(false);
    expect(useSettings.getState().settings?.overlay.font_size).toBe(40);

    finish();
    await inFlight;
    unsub();
  });
});
