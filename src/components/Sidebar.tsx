import { NavLink } from "react-router";
import { useTranslation } from "react-i18next";
import { Icon, type IconName } from "./icons";
import { useSession } from "../lib/session";
import { useSettings } from "../lib/settings";
import { useModels } from "../lib/models";

interface Item { to: string; icon: IconName; label: string }

export function Sidebar({ collapsed, onToggle }: { collapsed: boolean; onToggle: () => void }) {
  const { t } = useTranslation();
  const capturing = useSession((s) => s.capturing);
  const modelId = useSettings((s) => s.settings?.asr.model_id);
  const modelName = useModels((s) => s.models.find((m) => m.info.id === modelId)?.info.name ?? modelId);

  const main: Item[] = [
    { to: "/live", icon: "live", label: t("nav.live") },
    { to: "/history", icon: "history", label: t("nav.history") },
  ];
  const settings: Item[] = [
    { to: "/settings/general", icon: "general", label: t("settings.general") },
    { to: "/settings/models", icon: "models", label: t("settings.models") },
    { to: "/settings/translation", icon: "translation", label: t("settings.translation") },
    { to: "/settings/overlay", icon: "overlay", label: t("settings.overlay") },
  ];
  const cls = ({ isActive }: { isActive: boolean }) =>
    `flex items-center gap-2 rounded-full py-2 text-sm ${isActive ? "bg-surface-2 font-semibold text-fg" : "text-fg-muted hover:text-fg"} ${collapsed ? "justify-center" : "px-3"}`;
  const render = (i: Item) => (
    <NavLink key={i.to} to={i.to} className={cls} aria-label={i.label}>
      <Icon name={i.icon} />
      {!collapsed && <span>{i.label}</span>}
    </NavLink>
  );

  return (
    <aside className={`m-2 flex ${collapsed ? "w-14" : "w-52"} shrink-0 flex-col gap-0.5 rounded-[var(--radius-panel)] bg-base-2 p-2 shadow-[0_8px_24px_rgba(0,0,0,0.5)] transition-[width]`}>
      <div className={`mb-2 flex items-center ${collapsed ? "justify-center" : "justify-between"} px-2 pt-1`}>
        <span className="flex items-center gap-2 font-bold"><span className="h-5 w-5 rounded-md bg-accent" />{!collapsed && t("app.name")}</span>
        {!collapsed && (
          <button type="button" onClick={onToggle} aria-label={t("nav.collapse")} className="rounded-full p-1 text-fg-muted hover:bg-surface hover:text-fg"><Icon name="collapse" /></button>
        )}
      </div>
      {main.map(render)}
      {!collapsed && <div className="px-3 pb-1 pt-3 text-[10px] font-semibold uppercase tracking-wider text-fg-muted">{t("nav.settings")}</div>}
      {collapsed && <div className="my-1 h-px bg-surface" />}
      {settings.map(render)}
      {collapsed && (
        <button type="button" onClick={onToggle} aria-label={t("nav.expand")} className="mt-2 rounded-full p-2 text-fg-muted hover:bg-surface hover:text-fg"><Icon name="collapse" className="h-4 w-4 rotate-180" /></button>
      )}
      <div className={`mt-auto flex items-center gap-2 rounded-[var(--radius-card)] bg-base px-3 py-2 text-xs text-fg-muted ${collapsed ? "justify-center" : ""}`}>
        <span className={`h-2 w-2 shrink-0 rounded-full ${capturing ? "bg-accent" : "bg-fg-muted"}`} />
        {!collapsed && <span className="truncate">{modelName}</span>}
      </div>
    </aside>
  );
}
