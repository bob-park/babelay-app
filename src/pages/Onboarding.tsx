import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { DownloadToast } from "../components/DownloadToast";
import { ErrorBar } from "../components/ErrorBar";
import { Icon } from "../components/icons";
import { ModelRow } from "../components/ModelRow";
import { PermissionIcon, PermissionRow, type Perm } from "../components/PermissionRow";
import { SettingGroup } from "../components/SettingGroup";
import { useModels } from "../lib/models";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";
import type { ModelKind, UiLang } from "../lib/types";

type Step = "language" | "permission" | "asr" | "llm" | "done";
const ALL: Step[] = ["language", "permission", "asr", "llm", "done"];

export default function Onboarding() {
  const { t } = useTranslation();
  const { settings, update, setError } = useSettings();
  const { models, enqueue, refresh, queue, dequeue } = useModels();
  const [steps, setSteps] = useState<Step[]>(ALL);
  const [idx, setIdx] = useState(0);
  const [skippedLlm, setSkippedLlm] = useState(false);
  // 권한 단계가 조회한 값. 완료 단계에서 다시 탭을 만들지 않는다.
  const [perm, setPerm] = useState<Perm | null>(null);

  useEffect(() => {
    api.getPlatform().then((p) => { if (p !== "macos") setSteps((s) => s.filter((x) => x !== "permission")); }).catch(() => {});
    refresh();
  }, []);

  const last = steps.length - 1;
  const cur = Math.min(idx, last);
  const step = steps[cur];
  const next = () => setIdx(Math.min(cur + 1, last));
  const back = () => setIdx(Math.max(cur - 1, 0));

  if (!settings) return null;
  const byId = (id: string) => models.find((m) => m.info.id === id);
  const asr = byId(settings.asr.model_id);
  const llm = byId(settings.translation.local_model);
  const macos = steps.includes("permission");
  // 이미 설치된 모델을 건너뛰어도 준비된 건 준비된 거다.
  const llmSkipped = skippedLlm && !llm?.installed;

  // 모델 단계의 "다음": 미설치면 뒤에서 받기 시작하고 바로 넘어간다.
  const nextFromModel = (kind: ModelKind) => {
    const chosen = kind === "asr" ? asr : llm;
    if (chosen && !chosen.installed && !chosen.download) enqueue(chosen.info.id);
    if (kind === "llm") setSkippedLlm(false);
    next();
  };
  const select = (kind: ModelKind, id: string) =>
    (kind === "asr" ? update({ asr: { model_id: id } }) : update({ translation: { local_model: id } })).then(() => refresh());
  const rows = (kind: ModelKind) => {
    const current = kind === "asr" ? settings.asr.model_id : settings.translation.local_model;
    return models.filter((m) => m.info.kind === kind).map((m) => (
      <ModelRow key={m.info.id} status={m} selected={current === m.info.id} onSelect={() => select(kind, m.info.id)} />
    ));
  };
  const langBtn = (v: UiLang, text: string) => (
    <button key={v} type="button" className={`btn btn-sm ${settings.general.ui_language === v ? "btn-primary" : "btn-neutral"}`} onClick={() => update({ general: { ui_language: v } })}>{text}</button>
  );
  const pct = (m: typeof asr) => (m?.download ? `${Math.round((m.download.received / Math.max(1, m.download.total)) * 100)}%` : null);
  // 대기열에 있는 모델은 아직 실패가 아니다. ✕ 대신 중립 표시.
  const mark = (m: typeof asr) => m?.installed
    ? <span role="img" aria-label={t("permission.granted")} className="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-primary-content"><Icon name="check" /></span>
    : m?.download
      ? <span className="text-xs tabular-nums text-fg-muted">{pct(m)}</span>
      : m && queue.includes(m.info.id)
        ? <span role="img" aria-label={t("permission.unknown")} className="flex h-6 w-6 items-center justify-center rounded-full bg-neutral text-neutral-content"><Icon name="help" /></span>
        : <span role="img" aria-label={t("permission.denied")} className="flex h-6 w-6 items-center justify-center rounded-full bg-error text-error-content"><Icon name="x" /></span>;

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <DownloadToast />
      <ul className="steps w-full text-xs">
        {steps.map((s, i) => (
          <li key={s} className={`step ${i <= cur ? "step-primary" : ""}`} data-content={i < cur ? "✓" : String(i + 1)}>{t(`onboarding.step.${s}`)}</li>
        ))}
      </ul>
      <ErrorBar />
      <div className="mx-auto flex w-full max-w-2xl flex-1 flex-col gap-4 overflow-hidden">
        <h2 className="text-2xl font-bold">{t(`onboarding.title.${step}`)}</h2>

        <div className="flex flex-1 flex-col gap-2 overflow-auto">
          {step === "language" && <div className="flex flex-wrap gap-2">{langBtn("system", t("general.langSystem"))}{langBtn("ko", "한국어")}{langBtn("en", "English")}{langBtn("ja", "日本語")}</div>}
          {step === "permission" && <PermissionRow onStatus={setPerm} />}
          {step === "asr" && rows("asr")}
          {step === "llm" && rows("llm")}
          {step === "done" && (
            <SettingGroup>
              <div className="flex items-center justify-between px-4 py-3 text-sm"><span>{t("onboarding.check.asr")} · {asr?.info.name ?? "—"}</span>{mark(asr)}</div>
              <div className="flex items-center justify-between px-4 py-3 text-sm">
                <span>{t("onboarding.check.llm")} · {llmSkipped ? t("onboarding.skipped") : llm?.info.name ?? "—"}</span>
                {llmSkipped ? <span className="text-xs text-fg-muted">—</span> : mark(llm)}
              </div>
              {macos && <div className="flex items-center justify-between px-4 py-3 text-sm"><span>{t("permission.name")}</span><PermissionIcon perm={perm} /></div>}
            </SettingGroup>
          )}
        </div>

        <div className="flex items-center justify-between">
          <button type="button" className="btn btn-ghost btn-sm" onClick={back} disabled={cur === 0}>{t("onboarding.back")}</button>
          <div className="flex gap-2">
            {step === "llm" && <button type="button" className="btn btn-outline btn-sm" onClick={() => { if (llm) dequeue(llm.info.id); setSkippedLlm(true); next(); }}>{t("onboarding.skip")}</button>}
            {(step === "language" || step === "permission") && <button type="button" className="btn btn-primary btn-sm" onClick={next}>{t("onboarding.next")}</button>}
            {step === "asr" && <button type="button" className="btn btn-primary btn-sm" disabled={!asr} onClick={() => nextFromModel("asr")}>{t("onboarding.next")}</button>}
            {step === "llm" && <button type="button" className="btn btn-primary btn-sm" disabled={!llm} onClick={() => nextFromModel("llm")}>{t("onboarding.next")}</button>}
            {step === "done" && <button type="button" className="btn btn-primary btn-sm" disabled={!asr?.installed} onClick={() => api.finishOnboarding().catch(setError)}>{t("onboarding.finish")}</button>}
          </div>
        </div>
      </div>
    </div>
  );
}
