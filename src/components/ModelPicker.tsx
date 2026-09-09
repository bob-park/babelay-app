import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { formatSize, report, useModels } from "../lib/models";
import { matchPreset, presetState } from "../lib/presets";
import { useSettings } from "../lib/settings";
import { api } from "../lib/tauri";
import type { ModelKind, PresetId, PresetStatus } from "../lib/types";
import { ModelRow } from "./ModelRow";
import { SegmentedControl } from "./SegmentedControl";

interface Props {
  /** 적용 버튼 문구. 없으면 "…으로 설정하고 받기" / "…으로 설정". */
  applyLabel?: string;
  /** 적용이 끝난 뒤(설정 저장 + 큐 등록) 호출. 온보딩은 다음 단계로 넘어간다. */
  onApplied?: () => void;
}

// 프리셋 카드 3개 + 접힌 직접 선택. 설정 › 모델과 온보딩이 같이 쓴다.
export function ModelPicker({ applyLabel, onApplied }: Props) {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const { models, refresh, enqueue } = useModels();
  const [presets, setPresets] = useState<PresetStatus[]>([]);
  const [kind, setKind] = useState<ModelKind>("asr");
  const [selected, setSelected] = useState<PresetId | null>(null);
  const [manual, setManual] = useState<boolean | null>(null);
  const [applying, setApplying] = useState(false);

  useEffect(() => { refresh(); api.getPresets().then(setPresets).catch(report); }, []);

  const asrId = settings?.asr.model_id ?? "";
  const llmId = settings?.translation.local_model ?? "";
  const matched = useMemo(() => matchPreset(presets, asrId, llmId), [presets, asrId, llmId]);
  // 초기 선택: 현재 설정과 일치하는 프리셋, 없으면 균형. 어느 것과도 안 맞으면 직접 고르기를 펼친 채 시작.
  useEffect(() => {
    if (presets.length === 0) return;
    if (selected === null) setSelected(matched ?? "balanced");
    if (manual === null) setManual(matched === null && asrId !== "");
  }, [presets, matched]);

  if (!settings) return null;
  const byId = (id: string) => models.find((m) => m.info.id === id);
  const name = (id: string) => byId(id)?.info.name ?? id;
  const current = presets.find((p) => p.id === selected);
  const currentState = current ? presetState(current, models) : { installed: false, inUse: false };

  const apply = async () => {
    if (!current) return;
    setApplying(true);
    try {
      await update({ asr: { model_id: current.asr }, translation: { local_model: current.llm } });
      for (const id of [current.asr, current.llm]) {
        const m = byId(id);
        if (m && !m.installed) await enqueue(id, { replaceKind: true });
      }
      await refresh();
      onApplied?.();
    } finally {
      setApplying(false);
    }
  };
  const select = (id: string) => (kind === "asr" ? update({ asr: { model_id: id } }) : update({ translation: { local_model: id } })).then(() => refresh());
  const currentManual = kind === "asr" ? asrId : llmId;
  const label = applyLabel ?? t(currentState.installed ? "models.presetSet" : "models.presetApply", { name: current ? t(`models.preset.${current.id}.name`) : "" });

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-2 sm:grid-cols-3" role="radiogroup" aria-label={t("models.presets")}>
        {presets.map((p) => {
          const st = presetState(p, models);
          const on = p.id === selected;
          return (
            <button
              key={p.id}
              type="button"
              role="radio"
              aria-checked={on}
              onClick={() => setSelected(p.id)}
              className={`flex flex-col items-start gap-1 rounded-box bg-base-200 px-4 py-3 text-left text-sm hover:bg-base-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${on ? "ring-[1.5px] ring-inset ring-primary" : ""}`}
            >
              <span className="flex flex-wrap items-center gap-1.5 font-semibold">
                {t(`models.preset.${p.id}.name`)}
                {p.id === "balanced" && <span className="badge badge-primary badge-sm">{t("models.badgeRecommended")}</span>}
                {st.inUse && <span className="badge badge-neutral badge-sm">{t("models.badgeInUse")}</span>}
                {st.installed && !st.inUse && <span className="badge badge-neutral badge-sm">{t("models.badgeInstalled")}</span>}
                {p.heavy && <span className="badge badge-warning badge-sm">{t("models.badgeHeavy")}</span>}
              </span>
              <span className="text-xs text-fg-muted">{t(`models.preset.${p.id}.desc`)}</span>
              <span className="mt-1 text-xs">{name(p.asr)}<br />{name(p.llm)}</span>
              <span className="text-xs text-fg-muted">{formatSize(p.total_bytes)}</span>
            </button>
          );
        })}
      </div>
      <div className="flex justify-end">
        <button type="button" className="btn btn-primary btn-sm" disabled={!current || applying} onClick={apply}>{label}</button>
      </div>

      <details open={manual ?? false} onToggle={(e) => setManual(e.currentTarget.open)} className="flex flex-col gap-2">
        <summary className="cursor-pointer text-sm text-fg-muted">{t("models.pickManually")}</summary>
        <div className="mt-2 flex flex-col gap-2">
          <SegmentedControl value={kind} onChange={setKind} options={[{ value: "asr", label: t("models.asr") }, { value: "llm", label: t("models.llm") }]} />
          {models.filter((m) => m.info.kind === kind).map((m) => (
            <ModelRow key={m.info.id} status={m} selected={currentManual === m.info.id} onSelect={() => { if (m.installed) select(m.info.id); }} />
          ))}
        </div>
      </details>
    </div>
  );
}
