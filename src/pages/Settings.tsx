import { Navigate, NavLink, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import { Icon } from "../components/icons";
import General from "./settings/General";
import Models from "./settings/Models";
import Translation from "./settings/Translation";
import Overlay from "./settings/Overlay";

const TABS = ["general", "models", "translation", "overlay"] as const;
type Tab = (typeof TABS)[number];
const isTab = (v: string | undefined): v is Tab => (TABS as readonly string[]).includes(v ?? "");

export default function Settings() {
  const { t } = useTranslation();
  const { tab } = useParams();
  if (!isTab(tab)) return <Navigate to="/settings/general" replace />;
  const body = { general: <General />, models: <Models />, translation: <Translation />, overlay: <Overlay /> }[tab];
  return (
    <div className="flex w-full max-w-3xl flex-col gap-4">
      <h2 className="text-2xl font-bold">{t("nav.settings")}</h2>
      <div role="tablist" className="tabs tabs-border">
        {TABS.map((k) => (
          <NavLink key={k} to={`/settings/${k}`} role="tab" className={({ isActive }) => `tab gap-1.5 ${isActive ? "tab-active" : ""}`}>
            <Icon name={k} />{t(`settings.${k}`)}
          </NavLink>
        ))}
      </div>
      {body}
    </div>
  );
}
