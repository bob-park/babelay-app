import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { ERROR_KEYS, report } from "./models";
import type { EngineEvent } from "./types";

export interface Final { id: number; text: string; lang: string; start_ms: number; end_ms: number; tgt?: string }
export interface Partial { text: string; lang: string; start_ms: number }

export interface SessionView {
  capturing: boolean;
  stopping: boolean;
  gpuFallback: boolean;
  lagging: boolean;
  partial: Partial | null;
  finals: Final[];
  /// 실행 중인 세션의 설정(설정 화면에서 바꿔도 안 흔들린다). idle 이면 null.
  modelId: string | null;
  sourceLang: string | null;
  lastEventAt: number;
  /// 마지막 final 이 도착한 시각. 오버레이가 번역을 기다리는 기준점.
  lastFinalAt: number;
}

/** mm:ss — 타임라인과 히스토리가 같은 시간 표기를 쓴다. */
export function clock(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
}

export const initialView: SessionView = {
  capturing: false,
  stopping: false,
  gpuFallback: false,
  lagging: false,
  partial: null,
  finals: [],
  modelId: null,
  sourceLang: null,
  lastEventAt: 0,
  lastFinalAt: 0,
};

// 오래된 조각은 화면에도 안 남고 히스토리 DB가 진실이다. 메모리는 500줄에서 끊는다.
const MAX_FINALS = 500;

export function reduce(v: SessionView, ev: EngineEvent): SessionView {
  const next = { ...v, lastEventAt: Date.now() };
  switch (ev.type) {
    case "started":
      // 새 세션은 빈 타임라인에서 시작한다. 지난 세션의 줄이 섞이면 시간축이 거짓말을 한다.
      return { ...next, capturing: true, stopping: false, gpuFallback: ev.gpu_fallback, lagging: false, partial: null, finals: [], modelId: ev.model_id, sourceLang: ev.source_lang };
    case "partial":
      return { ...next, partial: { text: ev.text, lang: ev.lang, start_ms: ev.start_ms } };
    case "final": {
      const { type, ...f } = ev;
      return { ...next, partial: null, lagging: false, lastFinalAt: next.lastEventAt, finals: [...v.finals, f].slice(-MAX_FINALS) };
    }
    case "translated": {
      // 번역은 같은 id 의 final 에 붙는다. 이미 잘려 나간 오래된 id 면 버린다.
      const i = v.finals.findIndex((f) => f.id === ev.id);
      if (i < 0) return next;
      const finals = v.finals.slice();
      finals[i] = { ...finals[i], tgt: ev.text };
      return { ...next, finals };
    }
    case "lagging":
      return { ...next, lagging: true };
    case "stopped":
      return { ...next, capturing: false, stopping: false, gpuFallback: false, lagging: false, partial: null };
    case "error":
      // 배너는 bind가 띄운다. 시작이 실패했으면 Stopped 가 안 오므로 버튼 잠금은 여기서 푼다.
      return { ...next, stopping: false };
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
    // stopped가 올 때까지 버튼을 잠근다. 드레인 중 다시 누르면 busy_stopping이 난다.
    set({ view: { ...get().view, stopping: true } });
    try { await api.stopCapture(); } catch (e) { report(e); set({ view: { ...get().view, stopping: false } }); }
  },
  bind: () => {
    // 창이 늦게 붙으면 started를 놓친다. 현재 상태를 한 번 읽어 점을 맞춘다.
    // 그 사이 진짜 이벤트가 왔거나(lastEventAt) 언바인드됐으면 낡은 값이므로 버린다.
    let disposed = false;
    api.captureState().then((capturing) => {
      if (!disposed && get().view.lastEventAt === 0) set({ view: { ...get().view, capturing } });
    }).catch(() => {});
    const p = listen<EngineEvent>("engine-event", (e) => {
      const ev = e.payload;
      // code가 아는 코드면 code를, 아니면 message를 번역한다(둘 다 모르면 원문).
      if (ev.type === "error") report(ERROR_KEYS[ev.code] ? ev.code : ev.message);
      set({ view: reduce(get().view, ev) });
    });
    return () => { disposed = true; p.then((un) => un()); };
  },
}));
