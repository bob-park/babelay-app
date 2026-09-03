import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { PermissionRow } from "../../components/PermissionRow";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { resolveLang } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { api } from "../../lib/tauri";
import type { Theme, UiLang } from "../../lib/types";

const select = "select select-sm w-44";
const LANG_KEY = { ko: "general.langKo", en: "general.langEn", ja: "general.langJa" } as const;

export default function General() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const [platform, setPlatform] = useState("macos");
  useEffect(() => { api.getPlatform().then(setPlatform).catch(() => {}); }, []);
  if (!settings) return null;
  const systemLang = t(LANG_KEY[resolveLang("system", navigator.language)]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex justify-end">
        <div className="tooltip tooltip-left" data-tip={`${t("general.shortcutCapture")}: ⌘/Ctrl+Shift+S · ${t("general.shortcutOverlay")}: ⌘/Ctrl+Shift+O`}>
          <button type="button" className="btn btn-circle btn-ghost btn-sm" aria-label={t("general.shortcuts")}><Icon name="help" /></button>
        </div>
      </div>

      <SettingGroup>
        <SettingRow label={t("general.theme")}>
          <select className={select} value={settings.general.theme} onChange={(e) => update({ general: { theme: e.target.value as Theme } })}>
            <option value="system">{t("general.themeSystem")}</option>
            <option value="dark">{t("general.themeDark")}</option>
            <option value="light">{t("general.themeLight")}</option>
          </select>
        </SettingRow>
        <SettingRow label={t("general.language")}>
          <select className={select} value={settings.general.ui_language} onChange={(e) => update({ general: { ui_language: e.target.value as UiLang } })}>
            <option value="system">{`${t("general.langSystem")} (${systemLang})`}</option>
            <option value="ko">{t("general.langKo")}</option>
            <option value="en">{t("general.langEn")}</option>
            <option value="ja">{t("general.langJa")}</option>
          </select>
        </SettingRow>
      </SettingGroup>

      {platform === "macos" && (
        <>
          <div className="text-xs font-semibold uppercase tracking-wider text-fg-muted">{t("general.permission")}</div>
          <PermissionRow />
        </>
      )}
    </div>
  );
}
