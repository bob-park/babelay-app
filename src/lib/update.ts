import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./tauri";
import { useSettings } from "./settings";
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

// 업데이트 오류도 설정 오류 배너 하나로 보여준다. 배너를 하나 더 만들 이유가 없다.
const fail = (e: unknown) => useSettings.getState().setError(e);

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
    set({ progress: { received: 0, total: null } });
    try {
      await api.installUpdate();
    } catch (e) {
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
        // 설치 중 두 번째 설치 요청("busy")은 사용자가 알 일이 아니다. 진행률도 그대로 둔다.
        if (e.payload === "busy") return;
        set({ progress: null });
        fail(e.payload);
      }),
    ];
    return () => { for (const p of subs) p.then((un) => un()); };
  },
}));
