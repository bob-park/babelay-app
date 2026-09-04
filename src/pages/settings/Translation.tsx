import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { SegmentedControl } from "../../components/SegmentedControl";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { ERROR_KEYS, report, useModels } from "../../lib/models";
import { useSettings } from "../../lib/settings";
import { api } from "../../lib/tauri";
import type { Provider, SourceLang, TestTranslationResult, UiLang } from "../../lib/types";

const input = "input input-sm w-56";
const PROVIDERS: Provider[] = ["openai", "anthropic", "gemini", "deepl", "custom"];
// 테스트 결과 배너는 잠시만 보여준다.
const RESULT_MS = 5000;

export default function Translation() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const models = useModels((s) => s.models);
  const provider = settings?.translation.cloud.provider ?? "openai";
  const backend = settings?.translation.backend;
  const [key, setKey] = useState("");
  const [saved, setSaved] = useState(false);
  // 저장된 키를 새 값으로 덮어쓰는 중. 프로바이더가 바뀌면 접는다.
  const [editing, setEditing] = useState(false);
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; text: string } | null>(null);

  // 프로바이더마다 키가 따로 있다. 바뀌면 다시 묻고, 입력 중이던 키는 버린다.
  useEffect(() => {
    if (backend !== "cloud") return;
    setKey("");
    setEditing(false);
    let alive = true;
    api.hasApiKey(provider).then((v) => { if (alive) setSaved(v); }).catch(() => {});
    return () => { alive = false; };
  }, [provider, backend]);

  useEffect(() => {
    if (!result) return;
    const id = window.setTimeout(() => setResult(null), RESULT_MS);
    return () => window.clearTimeout(id);
  }, [result]);

  if (!settings) return null;
  const tr = settings.translation;
  const localName = tr.local_model ? models.find((m) => m.info.id === tr.local_model)?.info.name ?? tr.local_model : "—";

  const saveKey = () => {
    api.setApiKey(provider, key)
      .then(() => { setKey(""); setEditing(false); return api.hasApiKey(provider); })
      .then(setSaved)
      .catch(report);
  };
  const deleteKey = () => api.deleteApiKey(provider).then(() => setSaved(false)).catch(report);

  // 코드는 번역하고, 상세가 있으면 뒤에 붙인다.
  const failText = (code: string | null, detail: string) => {
    const k = code ? ERROR_KEYS[code] : undefined;
    const head = k ? t(k) : code ?? "";
    return detail && detail !== head ? (head ? `${head} · ${detail}` : detail) : head;
  };
  const test = () => {
    setTesting(true);
    api.testTranslation()
      .then((r: TestTranslationResult) => setResult(r.ok
        ? { ok: true, text: t("translation.testResult", { ms: r.ms, text: r.text }) }
        : { ok: false, text: failText(r.error, r.text) }))
      .catch((e: unknown) => { const m = e instanceof Error ? e.message : String(e); setResult({ ok: false, text: failText(m, "") }); })
      .finally(() => setTesting(false));
  };

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
          <SettingRow as="div" label={t("translation.apiKey")}>
            {saved && !editing ? (
              <>
                <span className="badge badge-neutral">{t("translation.saved")}</span>
                <button type="button" className="btn btn-ghost btn-sm" onClick={() => setEditing(true)}>{t("translation.changeKey")}</button>
                <button type="button" className="btn btn-ghost btn-sm" onClick={deleteKey}>{t("translation.deleteKey")}</button>
              </>
            ) : (
              <>
                <input
                  type="password"
                  autoComplete="off"
                  aria-label={t("translation.apiKey")}
                  className={input}
                  value={key}
                  onChange={(e) => setKey(e.target.value)}
                  onKeyDown={(e) => { if (e.key === "Enter" && key.trim()) saveKey(); }}
                />
                <button type="button" className="btn btn-sm btn-primary" disabled={!key.trim()} onClick={saveKey}>{t("translation.save")}</button>
                {editing && <button type="button" className="btn btn-ghost btn-sm" onClick={() => { setKey(""); setEditing(false); }}>{t("common.cancel")}</button>}
              </>
            )}
          </SettingRow>
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

      <div className="flex items-center gap-3">
        <button type="button" className="btn btn-outline btn-sm" disabled={testing} onClick={test}>
          {testing && <span className="loading loading-spinner loading-xs" />}
          {t("translation.test")}
        </button>
        {result && (
          <div role="status" className={`alert py-1 text-sm ${result.ok ? "alert-success" : "alert-error"}`}>{result.text}</div>
        )}
      </div>
    </div>
  );
}
