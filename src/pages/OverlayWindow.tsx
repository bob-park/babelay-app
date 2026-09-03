import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import { overlayLines } from "../lib/overlay";
import { useSession } from "../lib/session";
import { useSettings } from "../lib/settings";

// 마지막 이벤트 후 이만큼 지나면 자막을 지운다.
const IDLE_MS = 6000;
// 물리 픽셀. 이보다 좁으면 글자 한 줄도 안 들어간다.
const MIN_WIDTH = 240;

/**
 * macOS 의 tao 는 startResizeDragging 을 지원하지 않는다(NotSupported).
 * 핸들에서 포인터를 잡고 이동량만큼 창 폭을 직접 바꾼다. 높이는 그대로.
 */
function startWidthResize(e: ReactPointerEvent<HTMLDivElement>) {
  if (e.button !== 0) return;
  e.stopPropagation();
  const handle = e.currentTarget;
  handle.setPointerCapture(e.pointerId);
  const win = getCurrentWindow();
  const startX = e.screenX;
  const scale = window.devicePixelRatio || 1;
  let raf = 0;
  let pending: number | null = null;
  win.innerSize().then(({ width: w0, height: h0 }) => {
    const move = (ev: PointerEvent) => {
      pending = Math.max(MIN_WIDTH, Math.round(w0 + (ev.screenX - startX) * scale));
      if (raf) return;
      raf = requestAnimationFrame(() => {
        raf = 0;
        if (pending !== null) win.setSize(new PhysicalSize(pending, h0)).catch(() => {});
      });
    };
    const up = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", up);
      handle.removeEventListener("pointercancel", up);
      api.overlayCommitPosition().catch(useSettings.getState().setError);
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
    handle.addEventListener("pointercancel", up);
  });
}

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
  const { font_size, bg_opacity, display_mode } = settings.overlay;
  const last = view.finals[view.finals.length - 1]?.text ?? "";
  const partial = view.partial?.text ?? "";
  // 조정 모드에서 자막이 없으면 예시 문구로 상자를 채운다. 캡처 중이면 실제 자막이 우선.
  const sample = adjust && !view.capturing;
  const lines = sample
    ? overlayLines(display_mode, t("overlay.previewSource"), "", t("overlay.previewTarget"))
    : overlayLines(display_mode, last, partial);
  // 정지 뒤에는 마지막 자막이 다시 뜨면 안 된다(캡처 중일 때만 보인다).
  const visible = adjust || (view.capturing && fresh && Boolean(lines.primary || lines.secondary));

  return (
    <div className="flex h-full w-full items-end justify-center bg-transparent p-2">
      <div
        onMouseDown={(e) => { if (adjust && e.button === 0) getCurrentWindow().startDragging(); }}
        className={`relative rounded-[10px] px-4 py-2 text-center text-white transition-opacity duration-500 ${adjust ? "w-full min-h-12 cursor-move ring-2 ring-primary" : "max-w-full"}`}
        style={{ background: `rgba(18,18,18,${bg_opacity})`, backdropFilter: "blur(6px)", opacity: visible ? 1 : 0 }}
      >
        {lines.primary && <div style={{ fontSize: font_size, lineHeight: 1.3 }} className="font-bold">{lines.primary}</div>}
        {lines.secondary && <div style={{ fontSize: font_size * 0.6 }} className="text-white/70">{lines.secondary}</div>}
        {adjust && (
          <>
            <div className="absolute -top-6 left-0 rounded bg-primary px-2 py-0.5 text-xs font-bold text-primary-content">{t("overlay.adjustHint")}</div>
            <div
              onPointerDown={startWidthResize}
              onMouseDown={(e) => e.stopPropagation()}
              className="absolute -right-1.5 -bottom-1.5 h-3 w-3 cursor-ew-resize rounded-[2px] bg-primary"
            />
          </>
        )}
      </div>
    </div>
  );
}
