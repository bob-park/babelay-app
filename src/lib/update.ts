import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { showError } from "./toast";
import type { UpdateInfo, UpdateProgress } from "./types";

// 확인은 Rust 가 주기적으로 돈다. 여기는 결과 표시와 버튼뿐.
interface UpdateStore {
  info: UpdateInfo | null;
  progress: UpdateProgress | null;
  checking: boolean;
  checked: boolean; // 수동 확인을 끝낸 뒤에만 "최신 버전" 을 말한다
  check: () => Promise<void>;
  install: () => Promise<void>;
  subscribe: () => () => void;
}

// 업데이트 오류도 다른 오류와 같은 토스트로 보여준다.
const fail = showError;

// 설치 중 두 번째 설치 요청. 사용자가 알 일이 아니니 토스트도 진행률도 건드리지 않는다.
// invoke 는 문자열로, 이벤트는 payload 로 같은 "busy" 를 준다.
const isBusy = (e: unknown) => (e instanceof Error ? e.message : String(e)) === "busy";

export const useUpdate = create<UpdateStore>((set) => ({
  info: null,
  progress: null,
  checking: false,
  checked: false,
  check: async () => {
    set({ checking: true });
    try {
      set({ info: await api.checkUpdate(), checked: true });
    } catch (e) {
      fail(e);
    } finally {
      set({ checking: false });
    }
  },
  install: async () => {
    // 이미 받는 중이면 그 진행률을 유지한다. 0 으로 되돌리면 두 번째 클릭이 막대를 되감는다.
    set((s) => ({ progress: s.progress ?? { received: 0, total: null } }));
    try {
      await api.installUpdate();
    } catch (e) {
      if (isBusy(e)) return;
      set({ progress: null });
      fail(e);
    }
  },
  subscribe: () => {
    // 창이 이벤트보다 늦게 열렸을 수 있다.
    api.updateStatus().then((info) => set({ info })).catch(() => {});
    const subs = [
      listen<UpdateInfo>("update-available", (e) => set({ info: e.payload })),
      listen<UpdateProgress>("update-progress", (e) => set({ progress: e.payload })),
      listen<string>("update-error", (e) => {
        if (isBusy(e.payload)) return;
        set({ progress: null });
        fail(e.payload);
      }),
    ];
    return () => { for (const p of subs) p.then((un) => un()).catch(() => {}); };
  },
}));
