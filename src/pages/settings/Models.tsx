import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ModelRow } from "../../components/ModelRow";
import { SegmentedControl } from "../../components/SegmentedControl";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { useModels } from "../../lib/models";
import { useSettings } from "../../lib/settings";
import { api } from "../../lib/tauri";
import { HwBadges } from "../../components/SessionBadges";
import type { HwInfo, ModelKind } from "../../lib/types";

export default function Models() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const { models, refresh } = useModels();
  const [kind, setKind] = useState<ModelKind>("asr");
  const [platform, setPlatform] = useState<string | null>(null);
  const [hw, setHw] = useState<HwInfo | null>(null);
  useEffect(() => { refresh(); api.getPlatform().then(setPlatform).catch(() => setPlatform("macos")); api.getHwInfo().then(setHw).catch(() => {}); }, []);
  if (!settings) return null;

  const current = kind === "asr" ? settings.asr.model_id : settings.translation.local_model;
  // 백엔드의 in_use 가 낡으면 배지와 버튼이 어긋난다. 저장 후 목록을 다시 읽는다.
  const select = (id: string) => (kind === "asr" ? update({ asr: { model_id: id } }) : update({ translation: { local_model: id } })).then(() => refresh());

  return (
    <div className="flex flex-col gap-4">
      {hw && (
        <div className="flex flex-wrap items-center gap-1"><HwBadges hw={hw} /></div>
      )}
      <SegmentedControl value={kind} onChange={setKind} options={[{ value: "asr", label: t("models.asr") }, { value: "llm", label: t("models.llm") }]} />
      <div className="flex flex-col gap-2">
        {models.filter((m) => m.info.kind === kind).map((m) => (
          <ModelRow key={m.info.id} status={m} selected={current === m.info.id} onSelect={() => { if (m.installed) select(m.info.id); }} />
        ))}
      </div>
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
