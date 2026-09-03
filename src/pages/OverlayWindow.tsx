import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import { useSession } from "../lib/session";
import { useSettings } from "../lib/settings";

// 마지막 이벤트 후 이만큼 지나면 자막을 지운다.
const IDLE_MS = 6000;

export default function OverlayWindow() {
  const { t } = useTranslation();
  const settings = useSettings((s) => s.settings);
  const view = useSession((s) => s.view);
  const [adjust, setAdjust] = useState(false);
  const [fresh, setFresh] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    const un = listen<boolean>("overlay-adjust-mode", (e) => setAdjust(e.payload));
    return () => { un.then((f) => f()); };
  }, []);

  useEffect(() => {
    if (!view.lastEventAt) return;
    setFresh(true);
    const id = window.setTimeout(() => setFresh(false), IDLE_MS);
    return () => window.clearTimeout(id);
  }, [view.lastEventAt]);

  useEffect(() => {
    if (!adjust) return;
    const win = getCurrentWindow();
    const commit = () => {
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(
        () => api.overlayCommitPosition().catch(useSettings.getState().setError),
        300,
      );
    };
    const subs = [win.onMoved(commit), win.onResized(commit)];
    return () => {
      window.clearTimeout(timer.current);
      subs.forEach((p) => p.then((f) => f()));
    };
  }, [adjust]);

  if (!settings) return null;
  const { font_size, bg_opacity } = settings.overlay;
  // 2단계에는 번역이 없다. 표시 모드가 target이어도 원문을 보여준다.
  const last = view.finals[view.finals.length - 1];
  const partial = view.partial?.text ?? "";
  // 정지 뒤에는 마지막 자막이 다시 뜨면 안 된다(캡처 중일 때만 보인다).
  const visible = adjust || (view.capturing && fresh && Boolean(last || partial));

  return (
    <div className="flex h-full w-full items-end justify-center bg-transparent p-2">
      <div
        onMouseDown={(e) => { if (adjust && e.button === 0) getCurrentWindow().startDragging(); }}
        className={`relative max-w-full rounded-[10px] px-4 py-2 text-center text-white transition-opacity duration-500 ${adjust ? "min-h-12 min-w-48 cursor-move ring-2 ring-primary" : ""}`}
        style={{ background: `rgba(18,18,18,${bg_opacity})`, backdropFilter: "blur(6px)", opacity: visible ? 1 : 0 }}
      >
        {last && <div style={{ fontSize: font_size, lineHeight: 1.3 }} className="font-bold">{last.text}</div>}
        {partial && <div style={{ fontSize: font_size * 0.6 }} className="text-white/70">{partial}</div>}
        {adjust && (
          <>
            <div className="absolute -top-6 left-0 rounded bg-primary px-2 py-0.5 text-xs font-bold text-primary-content">{t("overlay.adjustHint")}</div>
            <div
              onMouseDown={(e) => { e.stopPropagation(); if (e.button === 0) getCurrentWindow().startResizeDragging("SouthEast"); }}
              className="absolute -right-1.5 -bottom-1.5 h-3 w-3 cursor-nwse-resize rounded-[2px] bg-primary"
            />
          </>
        )}
      </div>
    </div>
  );
}
