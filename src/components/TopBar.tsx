import { useState } from "react";
import { NavLink, useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { Icon } from "./icons";
import { useModels } from "../lib/models";
import { useSession } from "../lib/session";
import { useSettings } from "../lib/settings";
import { useUpdate } from "../lib/update";
import logo from "../../assets/icon.svg";

export function TopBar() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { view, start, stop } = useSession();
  const modelId = useSettings((s) => s.settings?.asr.model_id);
  const asrInstalled = useModels((s) => s.models.some((m) => m.info.id === modelId && m.installed));
  const pending = useUpdate((s) => s.info !== null && s.progress === null);
  const [q, setQ] = useState("");

  const missing = !view.capturing && !asrInstalled;
  const label = view.stopping ? t("live.stopping") : view.capturing ? t("live.stop") : t("live.start");
  const tab = ({ isActive }: { isActive: boolean }) =>
    `btn btn-sm btn-ghost gap-1.5 ${isActive ? "bg-secondary text-secondary-content" : "text-fg-muted"}`;

  return (
    <header className="flex h-12 shrink-0 items-center gap-3 border-b border-base-300 px-4">
      <img src={logo} alt="" className="h-6 w-6 shrink-0" />
      <nav className="flex gap-0.5">
        <NavLink to="/live" className={tab}><Icon name="live" />{t("nav.live")}</NavLink>
        <NavLink to="/history" className={tab}><Icon name="history" />{t("nav.history")}</NavLink>
        <NavLink to="/settings/general" className={tab}>
          <Icon name="general" />{t("nav.settings")}
          {pending && <span role="status" className="h-1.5 w-1.5 rounded-full bg-primary" aria-label={t("update.pending")} />}
        </NavLink>
      </nav>
      <div className="flex-1" />
      <input
        value={q}
        onChange={(e) => setQ(e.target.value)}
        onKeyDown={(e) => { if (e.key === "Enter" && q.trim()) { navigate(`/history?q=${encodeURIComponent(q.trim())}`); setQ(""); } }}
        placeholder={t("nav.search")}
        aria-label={t("nav.search")}
        className="input input-sm w-44"
      />
      <div className={missing ? "tooltip tooltip-bottom" : ""} data-tip={missing ? t("errors.modelMissing") : undefined}>
        {/* 캡처 중: 표면색 버튼 + 무지개 링 + 파형. 정지 중에는 링을 끄고 파형을 멈춘다. */}
        <div className={view.capturing && !view.stopping ? "aura aura-rainbow block" : ""}>
          <button
            type="button"
            className={`btn btn-sm gap-1.5 ${view.capturing ? "btn-neutral" : "btn-primary"}`}
            disabled={view.stopping || missing}
            aria-label={label}
            onClick={() => (view.capturing ? stop() : start())}
          >
            {view.capturing
              ? <span aria-hidden="true" className={`eq-bars text-primary ${view.stopping ? "paused" : ""}`}><i /><i /><i /><i /></span>
              : <Icon name="play" className="h-3.5 w-3.5 fill-current" />}
            {label}
          </button>
        </div>
      </div>
    </header>
  );
}
