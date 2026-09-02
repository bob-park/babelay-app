import { NavLink, useLocation } from "react-router";
import { useTranslation } from "react-i18next";
import { useSession } from "../lib/session";
import { useSettings } from "../lib/settings";

const item = ({ isActive }: { isActive: boolean }) =>
  `block rounded-md px-3 py-1.5 text-sm ${isActive ? "bg-surface font-bold text-fg" : "text-fg-muted hover:text-fg"}`;

export function Sidebar({ collapsed, onToggle }: { collapsed: boolean; onToggle: () => void }) {
  const { t } = useTranslation();
  const capturing = useSession((s) => s.capturing);
  const model = useSettings((s) => s.settings?.asr.model_id);
  const inSettings = useLocation().pathname.startsWith("/settings");
  const w = collapsed ? "w-14" : "w-52";

  return (
    <aside className={`flex ${w} shrink-0 flex-col gap-1 border-r border-surface bg-base p-3 transition-[width]`}>
      <div className="mb-2 flex items-center justify-between">
        {!collapsed && <span className="font-bold text-accent">● {t("app.name")}</span>}
        <button
          onClick={onToggle}
          aria-label={collapsed ? t("nav.expand") : t("nav.collapse")}
          className="rounded-full p-1 text-fg-muted hover:bg-surface hover:text-fg"
        >
          {collapsed ? "»" : "«"}
        </button>
      </div>
      <NavLink to="/live" className={item} title={t("nav.live")}>{collapsed ? "▶" : t("nav.live")}</NavLink>
      <NavLink to="/history" className={item} title={t("nav.history")}>{collapsed ? "≡" : t("nav.history")}</NavLink>
      <NavLink to="/settings/general" className={item({ isActive: collapsed && inSettings })} title={t("nav.settings")}>{collapsed ? "⚙" : t("nav.settings")}</NavLink>
      {!collapsed && (
        <div className="ml-3 flex flex-col gap-0.5 text-xs">
          <NavLink to="/settings/general" className={item}>{t("settings.general")}</NavLink>
          <NavLink to="/settings/transcription" className={item}>{t("settings.transcription")}</NavLink>
          <NavLink to="/settings/translation" className={item}>{t("settings.translation")}</NavLink>
          <NavLink to="/settings/overlay" className={item}>{t("settings.overlay")}</NavLink>
        </div>
      )}
      <div className="mt-auto flex items-center gap-2 rounded-md bg-base-2 p-2 text-xs text-fg-muted">
        <span className={`h-2 w-2 rounded-full ${capturing ? "bg-accent" : "bg-fg-muted"}`} />
        {!collapsed && <span>{capturing ? t("status.capturing") : t("status.idle")} · {model}</span>}
      </div>
    </aside>
  );
}
