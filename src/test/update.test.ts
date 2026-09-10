import { describe, it, expect, vi, beforeEach } from "vitest";
import { useUpdate } from "../lib/update";
import { useSettings } from "../lib/settings";
import { api } from "../lib/tauri";
import type { UpdateInfo, UpdateProgress } from "../lib/types";

vi.mock("../lib/tauri", () => ({
  api: { updateStatus: vi.fn(), checkUpdate: vi.fn(), installUpdate: vi.fn() },
}));

const h = vi.hoisted(() => ({ listeners: {} as Record<string, (e: { payload: unknown }) => void> }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, cb: (e: { payload: unknown }) => void) => {
    h.listeners[name] = cb;
    return Promise.resolve(() => {});
  },
}));

const info: UpdateInfo = { version: "0.3.0", notes: "" };

beforeEach(() => {
  useUpdate.setState({ info: null, progress: null, checking: false, checked: false });
  useSettings.setState({ error: null });
  vi.mocked(api.updateStatus).mockResolvedValue(null);
});

describe("useUpdate", () => {
  it("picks up a backend-found update via event", async () => {
    useUpdate.getState().subscribe();
    await Promise.resolve();
    h.listeners["update-available"]({ payload: info });
    expect(useUpdate.getState().info).toEqual(info);
  });

  it("check stores the result and marks checked", async () => {
    vi.mocked(api.checkUpdate).mockResolvedValueOnce(null);
    await useUpdate.getState().check();
    expect(useUpdate.getState()).toMatchObject({ info: null, checked: true, checking: false });
  });

  it("check failure lands in the settings error bar", async () => {
    vi.mocked(api.checkUpdate).mockRejectedValueOnce(new Error("offline"));
    await useUpdate.getState().check();
    expect(useSettings.getState().error).toBe("offline");
    expect(useUpdate.getState().checking).toBe(false);
  });

  it("install shows progress from events and clears it on failure", async () => {
    useUpdate.getState().subscribe();
    await Promise.resolve();
    vi.mocked(api.installUpdate).mockRejectedValueOnce(new Error("sig"));
    const p = useUpdate.getState().install();
    const prog: UpdateProgress = { received: 10, total: 100 };
    h.listeners["update-progress"]({ payload: prog });
    expect(useUpdate.getState().progress).toEqual(prog);
    await p;
    expect(useUpdate.getState().progress).toBeNull();
    expect(useSettings.getState().error).toBe("sig");
  });

  it("tray-triggered errors also surface", async () => {
    useUpdate.getState().subscribe();
    await Promise.resolve();
    h.listeners["update-error"]({ payload: "no_update" });
    expect(useSettings.getState().error).toBe("no_update");
  });

  it("ignores a busy rejection from a second install click", async () => {
    const prog: UpdateProgress = { received: 10, total: 100 };
    useUpdate.setState({ progress: prog });
    vi.mocked(api.installUpdate).mockRejectedValueOnce(new Error("busy"));
    await useUpdate.getState().install();
    expect(useUpdate.getState().progress).toEqual(prog);
    expect(useSettings.getState().error).toBeNull();
  });

  it("ignores a raw busy string from invoke", async () => {
    const prog: UpdateProgress = { received: 10, total: 100 };
    useUpdate.setState({ progress: prog });
    vi.mocked(api.installUpdate).mockRejectedValueOnce("busy");
    await useUpdate.getState().install();
    expect(useUpdate.getState().progress).toEqual(prog);
    expect(useSettings.getState().error).toBeNull();
  });

  it("ignores busy while an install is running", async () => {
    useUpdate.getState().subscribe();
    await Promise.resolve();
    const prog: UpdateProgress = { received: 10, total: 100 };
    h.listeners["update-progress"]({ payload: prog });
    h.listeners["update-error"]({ payload: "busy" });
    expect(useUpdate.getState().progress).toEqual(prog);
    expect(useSettings.getState().error).toBeNull();
  });
});
