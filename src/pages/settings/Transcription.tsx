import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ModelRow } from "../../components/ModelRow";
import { Toggle } from "../../components/Toggle";
import { ASR_MODELS, BALANCED } from "../../lib/models.fixture";
import { api } from "../../lib/tauri";
import { useSettings } from "../../lib/settings";

export default function Transcription() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const [platform, setPlatform] = useState("macos");
  useEffect(() => { api.getPlatform().then(setPlatform); }, []);
  if (!settings) return null;

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <h2 className="text-2xl font-bold">{t("settings.transcription")}</h2>
      <div className="flex flex-col gap-2">
        {ASR_MODELS.map((m) => (
          <ModelRow
            key={m.id}
            model={m}
            selected={settings.asr.model_id === m.id}
            badges={{ balanced: m.id === BALANCED.asr, inUse: settings.asr.model_id === m.id }}
            onSelect={() => update({ asr: { model_id: m.id } })}
          />
        ))}
      </div>
      <div className="max-w-md rounded-lg bg-base-2 p-4">
        <Toggle
          checked={settings.asr.gpu}
          onChange={(v) => update({ asr: { gpu: v } })}
          label={platform === "windows" ? t("models.gpuWin") : t("models.gpuMac")}
        />
      </div>
    </div>
  );
}
