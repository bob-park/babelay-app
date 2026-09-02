import { useTranslation } from "react-i18next";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { useSettings } from "../../lib/settings";
import type { Theme, UiLang } from "../../lib/types";

const select = "rounded-full bg-surface px-3 py-1.5 text-sm text-fg";

export default function General() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  if (!settings) return null;

  return (
    <div className="flex max-w-xl flex-col gap-4">
      <h2 className="text-2xl font-bold">{t("settings.general")}</h2>

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
            <option value="system">{t("general.langSystem")}</option>
            <option value="ko">{t("general.langKo")}</option>
            <option value="en">{t("general.langEn")}</option>
            <option value="ja">{t("general.langJa")}</option>
          </select>
        </SettingRow>
      </SettingGroup>

      <SettingGroup>
        <SettingRow as="div" label={t("general.shortcutCapture")}><kbd>⌘/Ctrl+Shift+S</kbd></SettingRow>
        <SettingRow as="div" label={t("general.shortcutOverlay")}><kbd>⌘/Ctrl+Shift+O</kbd></SettingRow>
      </SettingGroup>
    </div>
  );
}
