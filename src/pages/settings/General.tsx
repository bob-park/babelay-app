import { useTranslation } from "react-i18next";
import { useSettings } from "../../lib/settings";
import type { Theme, UiLang } from "../../lib/types";

const select = "rounded-md bg-surface px-3 py-2 text-sm text-fg";
const label = "text-[10px] font-bold uppercase tracking-[1.2px] text-fg-muted";

export default function General() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  if (!settings) return null;

  return (
    <div className="flex max-w-md flex-col gap-6">
      <h2 className="text-2xl font-bold">{t("settings.general")}</h2>
      <div className="flex flex-col gap-1">
        <span className={label}>{t("general.theme")}</span>
        <select className={select} value={settings.general.theme} onChange={(e) => update({ general: { theme: e.target.value as Theme } })}>
          <option value="system">{t("general.themeSystem")}</option>
          <option value="dark">{t("general.themeDark")}</option>
          <option value="light">{t("general.themeLight")}</option>
        </select>
      </div>
      <div className="flex flex-col gap-1">
        <span className={label}>{t("general.language")}</span>
        <select className={select} value={settings.general.ui_language} onChange={(e) => update({ general: { ui_language: e.target.value as UiLang } })}>
          <option value="system">{t("general.langSystem")}</option>
          <option value="ko">{t("general.langKo")}</option>
          <option value="en">{t("general.langEn")}</option>
          <option value="ja">{t("general.langJa")}</option>
        </select>
      </div>
      <div className="flex flex-col gap-1">
        <span className={label}>{t("general.shortcuts")}</span>
        <div className="rounded-md bg-base-2 p-3 text-sm">
          <div className="flex justify-between"><span>{t("general.shortcutCapture")}</span><kbd>⌘/Ctrl+Shift+S</kbd></div>
          <div className="flex justify-between"><span>{t("general.shortcutOverlay")}</span><kbd>⌘/Ctrl+Shift+O</kbd></div>
        </div>
      </div>
    </div>
  );
}
