import { create } from "zustand";
import i18next from "i18next";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { useSettings } from "./settings";
import type { DownloadEvent, DownloadState, ModelStatus } from "./types";

export type RowAction = "download" | "cancel" | "select" | "delete";

export function rowAction(s: ModelStatus): RowAction {
  if (s.download) return "cancel";
  if (!s.installed) return "download";
  if (s.in_use) return "delete";
  return "select";
}

const MB = 1024 * 1024;
const GB = 1024 * MB;
export function formatSize(bytes: number): string {
  return bytes >= GB ? `${(bytes / GB).toFixed(1)} GB` : `${Math.round(bytes / MB)} MB`;
}

interface ModelsStore {
  models: ModelStatus[];
  lastEvent: { id: string; state: DownloadState } | null;
  refresh: () => Promise<void>;
  download: (id: string) => Promise<void>;
  cancel: (id: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  bind: () => () => void;
}

// 백엔드는 코드 문자열을 던진다. 아는 코드만 번역하고 나머지는 그대로 보여준다.
export const ERROR_KEYS: Record<string, string> = {
  busy: "errors.busy",
  in_use: "errors.inUse",
  "not downloading": "errors.notDownloading",
  "unknown model": "errors.unknownModel",
  unknown_model: "errors.unknownModel",
  model_missing: "errors.modelMissing",
  start_failed: "errors.startFailed",
  busy_stopping: "errors.busyStopping",
};

export const report = (e: unknown) => {
  const key = ERROR_KEYS[e instanceof Error ? e.message : String(e)];
  useSettings.getState().setError(key ? i18next.t(key) : e);
};

export const useModels = create<ModelsStore>((set, get) => ({
  models: [],
  lastEvent: null,
  refresh: async () => {
    try { set({ models: await api.getModels() }); } catch (e) { report(e); }
  },
  download: async (id) => {
    try { await api.downloadModel(id); await get().refresh(); } catch (e) { report(e); }
  },
  cancel: async (id) => {
    try { await api.cancelDownload(id); } catch (e) { report(e); }
  },
  remove: async (id) => {
    try { await api.deleteModel(id); await get().refresh(); } catch (e) { report(e); }
  },
  bind: () => {
    const p = listen<DownloadEvent>("model-download", (e) => {
      const { id, received, total, state, message } = e.payload;
      set({ lastEvent: { id, state } });
      if (state === "downloading") {
        set({ models: get().models.map((m) => (m.info.id === id ? { ...m, download: { received, total } } : m)) });
        return;
      }
      if (state === "error" && message) report(message);
      get().refresh();
    });
    return () => { p.then((un) => un()); };
  },
}));
