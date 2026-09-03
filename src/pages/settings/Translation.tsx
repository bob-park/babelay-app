import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { SegmentedControl } from "../../components/SegmentedControl";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { useModels } from "../../lib/models";
import { useSettings } from "../../lib/settings";
import type { Provider, SourceLang, UiLang } from "../../lib/types";

const input = "input input-sm w-56";
const PROVIDERS: Provider[] = ["openai", "anthropic", "gemini", "deepl", "custom"];

export default function Translation() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const models = useModels((s) => s.models);
  if (!settings) return null;
  const tr = settings.translation;
  const localName = models.find((m) => m.info.id === tr.local_model)?.info.name ?? tr.local_model;

  return (
    <div className="flex flex-col gap-4">
      <SegmentedControl
        value={tr.backend}
        onChange={(v) => update({ translation: { backend: v } })}
        options={[{ value: "local" as const, label: t("translation.local") }, { value: "cloud" as const, label: t("translation.cloud") }]}
      />

      {tr.backend === "local" ? (
        <SettingGroup>
          <SettingRow as="div" label={t("translation.currentModel")}>
            <span>{localName}</span>
            <Link to="/settings/models" className="underline underline-offset-2 hover:text-fg-muted">{t("translation.changeInModels")}</Link>
          </SettingRow>
        </SettingGroup>
      ) : (
        <SettingGroup>
          <SettingRow label={t("translation.provider")}>
            <select className="select select-sm w-56" value={tr.cloud.provider} onChange={(e) => update({ translation: { cloud: { provider: e.target.value as Provider } } })}>
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

      <SettingGroup>
        <SettingRow label={t("translation.sourceLang")}>
          <select className="select select-sm w-44" value={settings.asr.source_lang} onChange={(e) => update({ asr: { source_lang: e.target.value as SourceLang } })}>
            <option value="auto">{t("translation.auto")}</option><option value="ko">{t("general.langKo")}</option><option value="en">{t("general.langEn")}</option><option value="ja">{t("general.langJa")}</option>
          </select>
        </SettingRow>
        <SettingRow label={t("translation.targetLang")}>
          <select className="select select-sm w-44" value={settings.overlay.subtitle_lang} onChange={(e) => update({ overlay: { subtitle_lang: e.target.value as UiLang } })}>
            <option value="system">{t("general.langSystem")}</option><option value="ko">{t("general.langKo")}</option><option value="en">{t("general.langEn")}</option><option value="ja">{t("general.langJa")}</option>
          </select>
        </SettingRow>
      </SettingGroup>
    </div>
  );
}
