import { useTranslation } from "react-i18next";
import { ModelRow } from "../../components/ModelRow";
import { PillButton } from "../../components/PillButton";
import { BALANCED, LLM_MODELS } from "../../lib/models.fixture";
import { useSettings } from "../../lib/settings";
import type { Provider } from "../../lib/types";

const label = "text-[10px] font-bold uppercase tracking-[1.2px] text-fg-muted";
const input = "rounded-md bg-surface px-3 py-2 text-sm text-fg";
const PROVIDERS: Provider[] = ["openai", "anthropic", "gemini", "deepl", "custom"];

export default function Translation() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  if (!settings) return null;
  const tr = settings.translation;

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <h2 className="text-2xl font-bold">{t("settings.translation")}</h2>
      <div className="flex gap-2">
        <PillButton variant={tr.backend === "local" ? "primary" : "default"} onClick={() => update({ translation: { backend: "local" } })}>{t("translation.local")}</PillButton>
        <PillButton variant={tr.backend === "cloud" ? "primary" : "default"} onClick={() => update({ translation: { backend: "cloud" } })}>{t("translation.cloud")}</PillButton>
      </div>

      {tr.backend === "local" ? (
        <div className="flex flex-col gap-2">
          {LLM_MODELS.map((m) => (
            <ModelRow
              key={m.id}
              model={m}
              selected={tr.local_model === m.id}
              badges={{ balanced: m.id === BALANCED.llm, inUse: tr.local_model === m.id }}
              onSelect={() => update({ translation: { local_model: m.id } })}
            />
          ))}
        </div>
      ) : (
        <div className="grid max-w-md gap-4">
          <div className="flex flex-col gap-1">
            <span className={label}>{t("translation.provider")}</span>
            <select className={input} value={tr.cloud.provider} onChange={(e) => update({ translation: { cloud: { provider: e.target.value as Provider } } })}>
              {PROVIDERS.map((p) => <option key={p} value={p}>{t(`translation.provider${p[0].toUpperCase()}${p.slice(1)}`)}</option>)}
            </select>
          </div>
          {tr.cloud.provider !== "deepl" && (
            <div className="flex flex-col gap-1">
              <span className={label}>{t("translation.model")}</span>
              <input className={input} value={tr.cloud.model} onChange={(e) => update({ translation: { cloud: { model: e.target.value } } })} />
            </div>
          )}
          {tr.cloud.provider === "custom" && (
            <div className="flex flex-col gap-1">
              <span className={label}>{t("translation.baseUrl")}</span>
              <input className={input} placeholder="https://api.example.com/v1" value={tr.cloud.base_url} onChange={(e) => update({ translation: { cloud: { base_url: e.target.value } } })} />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
