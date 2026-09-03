import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { currentMonitor, getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import { awaitingTranslation, overlayLines, pairForOverlay } from "../lib/overlay";
import { useSession } from "../lib/session";
import { useSettings } from "../lib/settings";

// 마지막 이벤트 후 이만큼 지나면 자막을 지운다.
const IDLE_MS = 6000;
// CSS 픽셀. 이보다 좁으면 글자 한 줄도 안 들어간다(쓸 때 배율을 곱한다).
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
  // innerSize 는 IPC 라 늦게 온다. 그 전에 손을 떼면 아예 시작하지 않는다(리스너 누수 방지).
  let released = false;
  const abort = () => { released = true; };
  handle.addEventListener("pointerup", abort, { once: true });
  handle.addEventListener("pointercancel", abort, { once: true });
  Promise.all([win.innerSize(), currentMonitor()]).then(([{ width: w0, height: h0 }, mon]) => {
    if (released) return;
    // 0.2 는 overlay.rs 의 ratios_from 이 저장할 때 거는 하한과 같다. 더 좁게 끌면 커밋에서 되튄다.
    const floor = Math.max(Math.round(MIN_WIDTH * scale), Math.round((mon?.size.width ?? 0) * 0.2));
    const move = (ev: PointerEvent) => {
      pending = Math.max(floor, Math.round(w0 + (ev.screenX - startX) * scale));
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
      // 마지막 프레임이 밀렸을 수 있다. 커밋 전에 최종 폭을 확정한다.
      cancelAnimationFrame(raf);
      raf = 0;
      const commit = () => api.overlayCommitPosition().catch(useSettings.getState().setError);
      if (pending !== null) win.setSize(new PhysicalSize(pending, h0)).then(commit, commit);
      else commit();
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
  const [, setTick] = useState(0);
  const timer = useRef<number | undefined>(undefined);

  // 번역 타겟은 백엔드가 Started 로 알려준다(UI 언어로 다시 유추하면 OS 로케일과 어긋난다).
  const tgt = view.targetLang;
  const now = Date.now();
  const waiting = awaitingTranslation(view.finals, tgt, now, view.lastFinalAt);

  // 번역을 기다리는 동안만 100ms 마다 다시 그려 3초 상한이 화면에 반영되게 한다.
  useEffect(() => {
    if (!waiting) return;
    const id = window.setInterval(() => setTick((n) => n + 1), 100);
    return () => window.clearInterval(id);
  }, [waiting]);

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
  // 원문과 번역은 한 세트. 번역이 늦으면 직전 세트를 잠시 유지한다.
  const pair = pairForOverlay(view.finals, tgt, now, view.lastFinalAt);
  const partial = view.partial?.text ?? "";
  // 조정 모드에서 자막이 없으면 예시 문구로 상자를 채운다. 캡처 중이면 실제 자막이 우선.
  const sample = adjust && !view.capturing;
  const lines = sample
    ? overlayLines(display_mode, t("overlay.previewSource"), "", t("overlay.previewTarget"))
    : overlayLines(display_mode, pair.source, partial, pair.translated);
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
