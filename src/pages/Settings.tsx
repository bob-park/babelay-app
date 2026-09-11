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
    <div className="flex h-full w-full flex-col gap-4">
      <div role="tablist" className="tabs tabs-border">
        {TABS.map((k) => (
          <NavLink key={k} to={`/settings/${k}`} role="tab" className={({ isActive }) => `tab gap-1.5 ${isActive ? "tab-active text-primary" : ""}`}>
            <Icon name={k} />{t(`settings.${k}`)}
          </NavLink>
        ))}
      </div>
      {/* 탭은 고정, 본문만 스크롤. 스크롤 영역을 위로 늘리고 같은 만큼 패딩을 줘서 맨 위 툴팁이 잘리지 않게 한다. */}
      <div className="-mt-6 min-h-0 flex-1 overflow-auto pt-6">{body}</div>
    </div>
  );
}
