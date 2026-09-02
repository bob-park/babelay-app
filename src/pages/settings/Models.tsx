import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ModelRow } from "../../components/ModelRow";
import { SegmentedControl } from "../../components/SegmentedControl";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { Toggle } from "../../components/Toggle";
import { useModels } from "../../lib/models";
import { useSettings } from "../../lib/settings";
import { api } from "../../lib/tauri";
import type { ModelKind } from "../../lib/types";

export default function Models() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const { models, refresh } = useModels();
  const [kind, setKind] = useState<ModelKind>("asr");
  const [platform, setPlatform] = useState("macos");
  useEffect(() => { refresh(); api.getPlatform().then(setPlatform).catch(() => {}); }, []);
  if (!settings) return null;

  const current = kind === "asr" ? settings.asr.model_id : settings.translation.local_model;
  // 백엔드의 in_use 가 낡으면 배지와 버튼이 어긋난다. 저장 후 목록을 다시 읽는다.
  const select = (id: string) => (kind === "asr" ? update({ asr: { model_id: id } }) : update({ translation: { local_model: id } })).then(() => refresh());

  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <h2 className="text-2xl font-bold">{t("settings.models")}</h2>
      <SegmentedControl value={kind} onChange={setKind} options={[{ value: "asr", label: t("models.asr") }, { value: "llm", label: t("models.llm") }]} />
      <div className="flex flex-col gap-2">
        {models.filter((m) => m.info.kind === kind).map((m) => (
          <ModelRow key={m.info.id} status={m} selected={current === m.info.id} onSelect={() => { if (m.installed) select(m.info.id); }} />
        ))}
      </div>
      <SettingGroup>
        <SettingRow as="div" label={platform === "windows" ? t("models.gpuWin") : t("models.gpuMac")}>
          <Toggle checked={settings.asr.gpu} onChange={(v) => update({ asr: { gpu: v } })} />
        </SettingRow>
      </SettingGroup>
    </div>
  );
}
