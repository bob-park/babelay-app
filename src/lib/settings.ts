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
  error: string | null;
  load: () => Promise<void>;
  update: (patch: DeepPartial<Settings>) => Promise<void>;
  setError: (e: unknown) => void;
  clearError: () => void;
  subscribeBackend: () => () => void;
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

// 인플라이트 update 수와 그 패치의 합. settings-changed는 호출자에게도 되돌아오므로
// 아직 디스크에 닿지 않은 필드는 에코 위에 다시 덮어야 낙관적 상태가 살아남는다.
// 에코 자체를 버리면 그 사이 백엔드가 바꾼 필드(트레이 토글 등)를 놓친다.
let pending = 0;
let pendingPatch: DeepPartial<Settings> | null = null;

export const useSettings = create<SettingsStore>((set, get) => ({
  settings: null,
  error: null,
  load: async () => {
    try {
      set({ settings: await api.getSettings() });
    } catch (e) {
      // 설정을 못 읽어도 셸은 띄운다. 기본값 + 오류 배너.
      set({ settings: get().settings ?? defaultSettings, error: message(e) });
    }
  },
  update: async (patch) => {
    const prev = get().settings;
    set({ settings: mergeSettings(prev ?? defaultSettings, patch) });
    pending++;
    pendingPatch = pendingPatch ? merge(pendingPatch, patch) : patch;
    try {
      await api.patchSettings(patch);
    } catch (e) {
      set({ settings: prev, error: message(e) });
    } finally {
      pending--;
      if (pending === 0) pendingPatch = null;
    }
  },
  setError: (e) => set({ error: message(e) }),
  clearError: () => set({ error: null }),
  subscribeBackend: () => {
    const p = listen<Settings>("settings-changed", (e) => {
      set({ settings: pendingPatch ? mergeSettings(e.payload, pendingPatch) : e.payload });
    });
    return () => {
      p.then((un) => un());
    };
  },
}));
