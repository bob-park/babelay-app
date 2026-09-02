import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";

interface SessionStore {
  capturing: boolean;
  toggle: () => void;
  bind: () => () => void;
}

// ponytail: 2단계에서 engine-event를 받아 실제 캡처 상태로 교체한다.
export const useSession = create<SessionStore>((set, get) => ({
  capturing: false,
  toggle: () => set({ capturing: !get().capturing }),
  bind: () => {
    const p = listen("capture-toggle", () => get().toggle());
    return () => {
      p.then((un) => un());
    };
  },
}));
