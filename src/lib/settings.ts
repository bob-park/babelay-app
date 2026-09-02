import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import type { DeepPartial, Settings } from "./types";

export const defaultSettings: Settings = {
  version: 1,
  general: { theme: "system", ui_language: "system", onboarding_done: false },
  asr: { model_id: "small", gpu: true, source_lang: "auto" },
  translation: {
    backend: "local",
    local_model: "qwen3.5-2b",
    cloud: { provider: "openai", model: "gpt-4o-mini", base_url: "" },
  },
  overlay: {
    enabled: true,
    monitor_id: "",
    x_ratio: 0.5,
    y_ratio: 0.85,
    w_ratio: 0.6,
    display_mode: "both",
    subtitle_lang: "system",
    font_size: 24,
    bg_opacity: 0.8,
  },
};

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function merge<T>(base: T, patch: DeepPartial<T>): T {
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(patch as Record<string, unknown>)) {
    if (v === undefined) continue;
    out[k] = isObj(v) && isObj(out[k]) ? merge(out[k], v) : v;
  }
  return out as T;
}

export const mergeSettings = (base: Settings, patch: DeepPartial<Settings>): Settings => merge(base, patch);

interface SettingsStore {
  settings: Settings | null;
  load: () => Promise<void>;
  update: (patch: DeepPartial<Settings>) => Promise<void>;
  subscribeBackend: () => () => void;
}

// 인플라이트 update 수. settings-changed는 호출자에게도 되돌아오므로
// 저장이 끝나기 전에 도착한 에코는 낙관적 상태를 되돌릴 수 있다.
let pending = 0;

export const useSettings = create<SettingsStore>((set, get) => ({
  settings: null,
  load: async () => set({ settings: await api.getSettings() }),
  update: async (patch) => {
    const prev = get().settings;
    const next = mergeSettings(prev ?? defaultSettings, patch);
    set({ settings: next });
    pending++;
    try {
      await api.setSettings(next);
    } catch (e) {
      set({ settings: prev });
      throw e;
    } finally {
      pending--;
    }
  },
  subscribeBackend: () => {
    const p = listen<Settings>("settings-changed", (e) => {
      if (pending > 0) return;
      set({ settings: e.payload });
    });
    return () => {
      p.then((un) => un());
    };
  },
}));
