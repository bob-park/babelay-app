import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { useSettings } from "./settings";
import type { DownloadEvent, ModelStatus } from "./types";

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
  refresh: () => Promise<void>;
  download: (id: string) => Promise<void>;
  cancel: (id: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  bind: () => () => void;
}

const report = (e: unknown) => useSettings.getState().setError(e);

export const useModels = create<ModelsStore>((set, get) => ({
  models: [],
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
