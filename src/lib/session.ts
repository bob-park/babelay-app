import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { ERROR_KEYS, report } from "./models";
import type { EngineEvent } from "./types";

export interface Final { id: number; text: string; lang: string; start_ms: number; end_ms: number }
export interface Partial { text: string; lang: string; start_ms: number }

export interface SessionView {
  capturing: boolean;
  gpuFallback: boolean;
  lagging: boolean;
  partial: Partial | null;
  finals: Final[];
  lastEventAt: number;
}

/** mm:ss — 타임라인과 히스토리가 같은 시간 표기를 쓴다. */
export function clock(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
}

export const initialView: SessionView = {
  capturing: false,
  gpuFallback: false,
  lagging: false,
  partial: null,
  finals: [],
  lastEventAt: 0,
};

// 오래된 조각은 화면에도 안 남고 히스토리 DB가 진실이다. 메모리는 500줄에서 끊는다.
const MAX_FINALS = 500;

export function reduce(v: SessionView, ev: EngineEvent): SessionView {
  const next = { ...v, lastEventAt: Date.now() };
  switch (ev.type) {
    case "started":
      // 새 세션은 빈 타임라인에서 시작한다. 지난 세션의 줄이 섞이면 시간축이 거짓말을 한다.
      return { ...next, capturing: true, gpuFallback: ev.gpu_fallback, lagging: false, partial: null, finals: [] };
    case "partial":
      return { ...next, partial: { text: ev.text, lang: ev.lang, start_ms: ev.start_ms } };
    case "final": {
      const { type, ...f } = ev;
      return { ...next, partial: null, lagging: false, finals: [...v.finals, f].slice(-MAX_FINALS) };
    }
    case "lagging":
      return { ...next, lagging: true };
    case "stopped":
      return { ...next, capturing: false, lagging: false, partial: null };
    case "error":
      // 배너는 bind가 띄운다. 뷰는 이벤트 시각만 갱신한다.
      return next;
  }
}

interface SessionStore {
  view: SessionView;
  start: () => Promise<void>;
  stop: () => Promise<void>;
  bind: () => () => void;
}

export const useSession = create<SessionStore>((set, get) => ({
  view: initialView,
  start: async () => {
    try { await api.startCapture(); } catch (e) { report(e); }
  },
  stop: async () => {
    try { await api.stopCapture(); } catch (e) { report(e); }
  },
  bind: () => {
    // 창이 늦게 붙으면 started를 놓친다. 현재 상태를 한 번 읽어 점을 맞춘다.
    api.captureState().then((capturing) => set({ view: { ...get().view, capturing } })).catch(() => {});
    const p = listen<EngineEvent>("engine-event", (e) => {
      const ev = e.payload;
      // code가 아는 코드면 code를, 아니면 message를 번역한다(둘 다 모르면 원문).
      if (ev.type === "error") report(ERROR_KEYS[ev.code] ? ev.code : ev.message);
      set({ view: reduce(get().view, ev) });
    });
    return () => { p.then((un) => un()); };
  },
}));
