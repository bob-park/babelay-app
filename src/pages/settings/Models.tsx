import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ModelPicker } from "../../components/ModelPicker";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { useSettings } from "../../lib/settings";
import { api } from "../../lib/tauri";
import { HwBadges } from "../../components/SessionBadges";
import type { HwInfo } from "../../lib/types";

export default function Models() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const [platform, setPlatform] = useState<string | null>(null);
  const [hw, setHw] = useState<HwInfo | null>(null);
  useEffect(() => { api.getPlatform().then(setPlatform).catch(() => setPlatform("macos")); api.getHwInfo().then(setHw).catch(() => {}); }, []);
  if (!settings) return null;

  return (
    <div className="flex flex-col gap-4">
      {hw && (
        <div className="flex flex-wrap items-center gap-1"><HwBadges hw={hw} /></div>
      )}
      <ModelPicker />
      {platform && (
        <SettingGroup>
          <SettingRow label={platform === "windows" ? t("models.gpuWin") : t("models.gpuMac")}>
            <input type="checkbox" role="switch" className="toggle toggle-primary" checked={settings.asr.gpu} onChange={(e) => update({ asr: { gpu: e.target.checked } })} />
          </SettingRow>
        </SettingGroup>
      )}
    </div>
  );
}
