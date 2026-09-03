import { useRef, useState } from "react";
import { NavLink, useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { Icon, type IconName } from "./icons";
import { useModels } from "../lib/models";
import { useSession } from "../lib/session";
import { useSettings } from "../lib/settings";
import { api } from "../lib/tauri";
import type { HwInfo } from "../lib/types";

interface Item { to: string; icon: IconName; label: string }

export function Sidebar({ collapsed, onToggle }: { collapsed: boolean; onToggle: () => void }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { view, start, stop } = useSession();
  const modelId = useSettings((s) => s.settings?.asr.model_id);
  const asrInstalled = useModels((s) => s.models.some((m) => m.info.id === modelId && m.installed));
  const [q, setQ] = useState("");
  const [hw, setHw] = useState<HwInfo | null>(null);
  const about = useRef<HTMLDialogElement>(null);

  const items: Item[] = [
    { to: "/live", icon: "live", label: t("nav.live") },
    { to: "/history", icon: "history", label: t("nav.history") },
  ];
  // 수동 접힘은 항상 숨긴다. 자동은 800px 미만에서만.
  const label = collapsed ? "hidden" : "hidden wide:inline";
  const wide = collapsed ? "" : "wide:w-52";
  const justify = collapsed ? "" : "wide:justify-start";
  const navCls = ({ isActive }: { isActive: boolean }) =>
    `btn btn-sm justify-center gap-2 ${justify} ${isActive ? "btn-neutral" : "btn-ghost text-fg-muted"}`;
  const openAbout = () => { if (!hw) api.getHwInfo().then(setHw).catch(() => {}); about.current?.showModal(); };
  const hwLine = hw ? [hw.chip, `${hw.mem_gb} GB`, hw.gpu && `${hw.gpu}${hw.gpu_mem_gb ? ` ${hw.gpu_mem_gb} GB` : ""}`].filter(Boolean).join(" · ") : "";

  return (
    <aside className={`relative flex w-14 shrink-0 flex-col gap-1 border-r border-base-300 bg-base-200 p-2 ${wide}`}>
      <div className={`mb-1 flex items-center justify-center gap-2 px-1 pt-1 font-bold ${justify}`}>
        <span className="h-5 w-5 shrink-0 rounded-md bg-primary" />
        <span className={label}>{t("app.name")}</span>
      </div>
      {!collapsed && (
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter" && q.trim()) { navigate(`/history?q=${encodeURIComponent(q.trim())}`); setQ(""); } }}
          placeholder={t("nav.search")}
          aria-label={t("nav.search")}
          className="input input-sm mb-1 hidden w-full rounded-full wide:block"
        />
      )}
      {items.map((i) => (
        <NavLink key={i.to} to={i.to} className={navCls} aria-label={i.label}>
          <Icon name={i.icon} /><span className={label}>{i.label}</span>
        </NavLink>
      ))}
      <button
        type="button"
        onClick={onToggle}
        aria-label={collapsed ? t("nav.expand") : t("nav.collapse")}
        className="btn btn-circle btn-neutral btn-xs absolute -right-3 top-12 hidden wide:flex"
      >
        <Icon name="collapse" className={`h-3 w-3 ${collapsed ? "rotate-180" : ""}`} />
      </button>

      <div className="mt-auto flex flex-col gap-1">
        <button
          type="button"
          className="btn btn-primary btn-sm btn-block gap-1"
          disabled={view.stopping || (!view.capturing && !asrInstalled)}
          title={!view.capturing && !asrInstalled ? t("errors.modelMissing") : undefined}
          aria-label={view.stopping ? t("live.stopping") : view.capturing ? t("live.stop") : t("live.start")}
          onClick={() => (view.capturing ? stop() : start())}
        >
          <span aria-hidden="true">{view.capturing ? "■" : "●"}</span>
          <span className={label}>{view.stopping ? t("live.stopping") : view.capturing ? t("live.stop") : t("live.start")}</span>
        </button>
        <NavLink to="/settings/general" className={navCls} aria-label={t("nav.settings")}>
          <Icon name="general" /><span className={label}>{t("nav.settings")}</span>
        </NavLink>
        <button type="button" className={`btn btn-ghost btn-sm justify-center gap-2 text-fg-muted ${justify}`} onClick={openAbout} aria-label={t("nav.about")}>
          <Icon name="info" /><span className={label}>{t("nav.about")}</span>
        </button>
        <div className={`text-center text-[10px] text-fg-muted ${label}`}>v{import.meta.env.PACKAGE_VERSION}</div>
      </div>

      <dialog ref={about} className="modal">
        <div className="modal-box max-w-sm">
          <h3 className="text-lg font-bold">{t("app.name")}</h3>
          <div className="mt-2 text-sm"><span className="text-fg-muted">{t("about.version")}</span> {import.meta.env.PACKAGE_VERSION}</div>
          {hwLine && <div className="text-sm"><span className="text-fg-muted">{t("about.hardware")}</span> {hwLine}</div>}
          <div className="modal-action">
            <form method="dialog"><button className="btn btn-sm">{t("common.close")}</button></form>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop"><button aria-label={t("common.close")} /></form>
      </dialog>
    </aside>
  );
}
