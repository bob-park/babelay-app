import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";

export default function OverlayWindow() {
  const { t } = useTranslation();
  const settings = useSettings((s) => s.settings);
  const [adjust, setAdjust] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    const un = listen<boolean>("overlay-adjust-mode", (e) => setAdjust(e.payload));
    return () => { un.then((f) => f()); };
  }, []);

  useEffect(() => {
    if (!adjust) return;
    const win = getCurrentWindow();
    const commit = () => {
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => api.overlayCommitPosition(), 300);
    };
    const subs = [win.onMoved(commit), win.onResized(commit)];
    return () => {
      window.clearTimeout(timer.current);
      subs.forEach((p) => p.then((f) => f()));
    };
  }, [adjust]);

  if (!settings) return null;
  const { display_mode, font_size, bg_opacity } = settings.overlay;
  // ponytail: 2단계 전까지는 샘플 문장을 항상 표시한다.
  const source = t("overlay.sampleSource");
  const target = t("overlay.sampleTarget");

  return (
    <div className="flex h-full w-full items-end justify-center bg-transparent p-2">
      <div
        onMouseDown={(e) => { if (adjust && e.button === 0) getCurrentWindow().startDragging(); }}
        className={`relative max-w-full rounded-[10px] px-4 py-2 text-center text-white ${adjust ? "cursor-move ring-2 ring-accent" : ""}`}
        style={{ background: `rgba(18,18,18,${bg_opacity})`, backdropFilter: "blur(6px)" }}
      >
        {display_mode !== "target" && (
          <div style={{ fontSize: font_size * 0.6 }} className="text-white/70">{source}</div>
        )}
        {display_mode !== "source" && (
          <div style={{ fontSize: font_size, lineHeight: 1.3 }} className="font-bold">{target}</div>
        )}
        {adjust && (
          <>
            <div className="absolute -top-6 left-0 rounded bg-accent px-2 py-0.5 text-xs font-bold text-accent-fg">{t("overlay.adjustHint")}</div>
            <div
              onMouseDown={(e) => { e.stopPropagation(); if (e.button === 0) getCurrentWindow().startResizeDragging("SouthEast"); }}
              className="absolute -right-1.5 -bottom-1.5 h-3 w-3 cursor-nwse-resize rounded-[2px] bg-accent"
            />
          </>
        )}
      </div>
    </div>
  );
}
