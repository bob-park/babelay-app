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
/** 히스토리 `translator` 값(`local:<model>` / `cloud:<provider>/<model>`) → 배지 문구. 없으면 null. */
export function translatorLabel(translator: string | null | undefined, name: (id: string) => string): string | null {
  if (!translator) return null;
  if (translator.startsWith("local:")) return name(translator.slice(6));
  if (translator.startsWith("cloud:")) return translator.slice(6).replace("/", " · ");
  return translator;
}

export function formatSize(bytes: number): string {
  return bytes >= GB ? `${(bytes / GB).toFixed(1)} GB` : `${Math.round(bytes / MB)} MB`;
}

interface ModelsStore {
  models: ModelStatus[];
  lastEvent: { id: string; state: DownloadState } | null;
  queue: string[];
  refresh: () => Promise<void>;
  download: (id: string) => Promise<void>;
  enqueue: (id: string) => Promise<void>;
  dequeue: (id: string) => void;
  cancel: (id: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  bind: () => () => void;
}

// 백엔드는 코드 문자열을 던진다. 아는 코드만 번역하고 나머지는 그대로 보여준다.
export const ERROR_KEYS: Record<string, string> = {
  busy: "errors.busy",
  capturing: "errors.capturing",
  "not downloading": "errors.notDownloading",
  "unknown model": "errors.unknownModel",
  unknown_model: "errors.unknownModel",
  model_missing: "errors.modelMissing",
  start_failed: "errors.startFailed",
  busy_stopping: "errors.busyStopping",
  translation_model_missing: "errors.translationModelMissing",
  api_key_missing: "errors.apiKeyMissing",
  base_url_missing: "errors.baseUrlMissing",
  translate: "errors.translateFailed",
  display_mode_source: "errors.displayModeSource",
  unknown_provider: "errors.unknownProvider",
  timeout: "errors.timeout",
};

export const report = (e: unknown) => {
  const key = ERROR_KEYS[e instanceof Error ? e.message : String(e)];
  useSettings.getState().setError(key ? i18next.t(key) : e);
};

// 모듈 스코프. download() 를 부른 뒤 첫 진행 이벤트가 오기 전까지의 창을 막는다.
let starting: string | null = null;

export const useModels = create<ModelsStore>((set, get) => ({
  models: [],
  lastEvent: null,
  refresh: async () => {
    try { set({ models: await api.getModels() }); } catch (e) { report(e); }
  },
  download: async (id) => {
    try { await api.downloadModel(id); await get().refresh(); } catch (e) { report(e); }
  },
  queue: [],
  enqueue: async (id) => {
    const s = get();
    const activeId = starting ?? s.models.find((m) => m.download)?.info.id ?? null;
    if (activeId === id) return;                 // 이미 받는 중이면 무시
    if (!activeId) {
      starting = id;
      try { await s.download(id); } finally { starting = null; }
      return;
    }
    const kindOf = (x: string) => s.models.find((m) => m.info.id === x)?.info.kind;
    const kind = kindOf(id);
    // 온보딩에서 마음을 바꾸면 같은 종류의 대기 항목은 새 선택으로 갈아끼운다.
    set({ queue: [...s.queue.filter((q) => q !== id && kindOf(q) !== kind), id] });
  },
  dequeue: (id) => set({ queue: get().queue.filter((q) => q !== id) }),
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
      get().refresh().then(() => {
        const [next, ...rest] = get().queue;
        if (!next) return;
        set({ queue: rest });
        get().enqueue(next);
      });
    });
    return () => { p.then((un) => un()); };
  },
}));
