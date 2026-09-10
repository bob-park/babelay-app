import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PermissionRow } from "../../components/PermissionRow";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { LANG_KEY, resolveLang } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { api } from "../../lib/tauri";
import { showError } from "../../lib/toast";
import { useUpdate } from "../../lib/update";
import type { Theme, UiLang } from "../../lib/types";

const select = "select select-sm w-44";

export default function General() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const [platform, setPlatform] = useState<string | null>(null);
  useEffect(() => { api.getPlatform().then(setPlatform).catch(() => {}); }, []);
  const { info, progress, checking, checked, check, install } = useUpdate();
  const [autostart, setAutostart] = useState<boolean | null>(null);
  useEffect(() => { api.getAutostart().then(setAutostart).catch(() => {}); }, []);
  const toggleAutostart = (on: boolean) => {
    setAutostart(on);
    api.setAutostart(on).catch((e) => { setAutostart(!on); showError(e); });
  };
  if (!settings) return null;
  const systemLang = t(LANG_KEY[resolveLang("system", navigator.language)]);
  const mod = platform === "macos" ? "⌘" : "Ctrl";
  const keys = (last: string) => <span className="flex gap-1">{[mod, "⇧", last].map((k) => <kbd key={k} className="kbd kbd-sm">{k}</kbd>)}</span>;

  return (
    <div className="flex flex-col gap-4">
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

      <div className="text-xs font-semibold uppercase tracking-wider text-fg-muted">{t("general.shortcuts")}</div>
      <SettingGroup>
        <SettingRow label={t("general.shortcutCapture")} as="div">{keys("S")}</SettingRow>
        <SettingRow label={t("general.shortcutOverlay")} as="div">{keys("O")}</SettingRow>
      </SettingGroup>

      <div className="text-xs font-semibold uppercase tracking-wider text-fg-muted">{t("general.updates")}</div>
      <SettingGroup>
        <SettingRow label={t("update.auto")}>
          <input type="checkbox" role="switch" className="toggle toggle-primary" checked={settings.general.auto_update} onChange={(e) => update({ general: { auto_update: e.target.checked } })} />
        </SettingRow>
        <SettingRow label={`${t("update.version")} ${import.meta.env.PACKAGE_VERSION}`} as="div">
          {progress ? (
            <span className="text-xs">{t("update.installing")}</span>
          ) : info ? (
            <>
              <span className="text-xs">{t("update.available", { version: info.version })}</span>
              <button type="button" className="btn btn-primary btn-sm" onClick={install}>{t("update.install")}</button>
            </>
          ) : (
            <>
              <span className="text-xs">{checking ? t("update.checking") : checked ? t("update.latest") : ""}</span>
              <button type="button" className="btn btn-sm" disabled={checking} onClick={check}>{t("update.check")}</button>
            </>
          )}
        </SettingRow>
        <SettingRow label={t("general.autostart")}>
          <input type="checkbox" role="switch" className="toggle toggle-primary" checked={autostart ?? false} disabled={autostart === null} onChange={(e) => toggleAutostart(e.target.checked)} />
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
