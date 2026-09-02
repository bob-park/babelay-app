import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { SegmentedControl } from "../../components/SegmentedControl";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { useModels } from "../../lib/models";
import { useSettings } from "../../lib/settings";
import type { Provider } from "../../lib/types";

const input = "rounded-md bg-surface px-3 py-2 text-sm text-fg";
const PROVIDERS: Provider[] = ["openai", "anthropic", "gemini", "deepl", "custom"];

export default function Translation() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const models = useModels((s) => s.models);
  if (!settings) return null;
  const tr = settings.translation;
  const localName = models.find((m) => m.info.id === tr.local_model)?.info.name ?? tr.local_model;

  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <h2 className="text-2xl font-bold">{t("settings.translation")}</h2>
      <SegmentedControl
        value={tr.backend}
        onChange={(v) => update({ translation: { backend: v } })}
        options={[{ value: "local" as const, label: t("translation.local") }, { value: "cloud" as const, label: t("translation.cloud") }]}
      />

      {tr.backend === "local" ? (
        <SettingGroup>
          <SettingRow as="div" label={t("translation.currentModel")}>
            <span className="text-fg">{localName}</span>
            <Link to="/settings/models" className="text-accent hover:underline">{t("translation.changeInModels")}</Link>
          </SettingRow>
        </SettingGroup>
      ) : (
        <SettingGroup>
          <SettingRow label={t("translation.provider")}>
            <select className={input} value={tr.cloud.provider} onChange={(e) => update({ translation: { cloud: { provider: e.target.value as Provider } } })}>
              {PROVIDERS.map((p) => <option key={p} value={p}>{t(`translation.provider${p[0].toUpperCase()}${p.slice(1)}`)}</option>)}
            </select>
          </SettingRow>
          {tr.cloud.provider !== "deepl" && (
            <SettingRow label={t("translation.model")}>
              <input className={input} value={tr.cloud.model} onChange={(e) => update({ translation: { cloud: { model: e.target.value } } })} />
            </SettingRow>
          )}
          {tr.cloud.provider === "custom" && (
            <SettingRow label={t("translation.baseUrl")}>
              <input className={input} placeholder="https://api.example.com/v1" value={tr.cloud.base_url} onChange={(e) => update({ translation: { cloud: { base_url: e.target.value } } })} />
            </SettingRow>
          )}
        </SettingGroup>
      )}
    </div>
  );
}
