import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ErrorBar } from "../components/ErrorBar";
import { ModelRow } from "../components/ModelRow";
import { formatSize, useModels } from "../lib/models";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";
import type { ModelKind, UiLang } from "../lib/types";

type Step = "language" | "permission" | "asr" | "llm" | "done";
const ALL: Step[] = ["language", "permission", "asr", "llm", "done"];

export default function Onboarding() {
  const { t } = useTranslation();
  const { settings, update, setError } = useSettings();
  const { models, download, refresh, lastEvent } = useModels();
  const [steps, setSteps] = useState<Step[]>(ALL);
  const [idx, setIdx] = useState(0);
  const [perm, setPerm] = useState<"granted" | "denied" | "unknown" | null>(null);
  const [waiting, setWaiting] = useState<string | null>(null); // 다운로드 완료를 기다리는 모델 id

  useEffect(() => {
    api.getPlatform().then((p) => { if (p !== "macos") setSteps((s) => s.filter((x) => x !== "permission")); }).catch(() => {});
    refresh();
  }, []);

  const last = steps.length - 1;
  const cur = Math.min(idx, last);
  const step = steps[cur];

  // 권한 조회는 실제 탭을 만들어 TCC 프롬프트를 띄운다. 그 단계에 왔을 때 딱 한 번만.
  useEffect(() => {
    if (step === "permission" && perm === null) api.checkAudioPermission().then(setPerm).catch(() => {});
  }, [step, perm]);

  const next = () => setIdx(Math.min(cur + 1, last));
  const back = () => { setWaiting(null); setIdx(Math.max(cur - 1, 0)); };

  // 지금 이 단계에서 고른 모델. 다른 모델을 고르거나 다른 단계로 가면 대기 중인 다운로드는 무효다.
  const chosenId = step === "asr" ? settings?.asr.model_id : step === "llm" ? settings?.translation.local_model : null;

  // 다운로드가 끝나 설치되면 자동으로 다음 단계
  useEffect(() => {
    if (waiting && waiting === chosenId && models.find((m) => m.info.id === waiting)?.installed) { setWaiting(null); next(); }
  }, [models, waiting, chosenId]);

  // 취소·실패로 끝났으면 버튼을 다시 살린다. 안 그러면 영영 disabled.
  useEffect(() => {
    if (waiting && lastEvent && lastEvent.id === waiting && lastEvent.state !== "downloading" && lastEvent.state !== "done") setWaiting(null);
  }, [lastEvent, waiting]);

  if (!settings) return null;

  const modelStep = (kind: ModelKind) => {
    const current = kind === "asr" ? settings.asr.model_id : settings.translation.local_model;
    const chosen = models.find((m) => m.info.id === current);
    const select = (id: string) => (kind === "asr" ? update({ asr: { model_id: id } }) : update({ translation: { local_model: id } })).then(() => refresh());
    const primary = !chosen ? null : chosen.installed
      ? <button type="button" className="btn btn-primary btn-sm" onClick={next}>{t("models.continue")}</button>
      : chosen.download
        ? <button type="button" className="btn btn-primary btn-sm" disabled>{`${Math.round((chosen.download.received / Math.max(1, chosen.download.total)) * 100)}%`}</button>
        : <button type="button" className="btn btn-primary btn-sm" disabled={waiting === chosen.info.id} onClick={() => { setWaiting(chosen.info.id); download(chosen.info.id); }}>{t("models.continueWith", { size: formatSize(chosen.info.size_bytes) })}</button>;
    const rows = models
      .filter((m) => m.info.kind === kind)
      .map((m) => <ModelRow key={m.info.id} status={m} selected={current === m.info.id} onSelect={() => select(m.info.id)} />);
    return { rows, primary };
  };

  const asr = step === "asr" ? modelStep("asr") : null;
  const llm = step === "llm" ? modelStep("llm") : null;
  const langBtn = (v: UiLang, text: string) => <button key={v} type="button" className={`btn btn-sm ${settings.general.ui_language === v ? "btn-primary" : "btn-neutral"}`} onClick={() => update({ general: { ui_language: v } })}>{text}</button>;

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <div className="flex gap-1.5">{steps.map((s, i) => <span key={s} className={`h-1 flex-1 rounded-full ${i <= cur ? "bg-primary" : "bg-base-300"}`} />)}</div>
      <ErrorBar />
      <h2 className="text-2xl font-bold">{t(`onboarding.title.${step}`)}</h2>

      <div className="flex flex-1 flex-col gap-2 overflow-auto">
        {step === "language" && <div className="flex flex-wrap gap-2">{langBtn("system", t("general.langSystem"))}{langBtn("ko", "한국어")}{langBtn("en", "English")}{langBtn("ja", "日本語")}</div>}
        {step === "permission" && (
          <div className="flex flex-col gap-3">
            <div className="flex gap-2">
              <button type="button" className={`btn btn-sm ${perm === "denied" ? "btn-neutral" : "btn-primary"}`} onClick={() => api.checkAudioPermission().then(setPerm).catch(setError)}>{t("onboarding.permissionCheck")}</button>
              <button type="button" className={`btn btn-sm ${perm === "denied" ? "btn-primary" : "btn-outline"}`} onClick={() => api.openPrivacySettings().catch(setError)}>{t("onboarding.openSettings")}</button>
            </div>
            {perm && <p className="text-sm text-fg-muted">{t(`onboarding.permission${perm[0].toUpperCase()}${perm.slice(1)}`)}</p>}
          </div>
        )}
        {asr?.rows}
        {llm?.rows}
      </div>

      <div className="flex items-center justify-between">
        <button type="button" className="btn btn-ghost btn-sm" onClick={back} disabled={cur === 0}>{t("onboarding.back")}</button>
        <div className="flex gap-2">
          {step === "llm" && <button type="button" className="btn btn-outline btn-sm" onClick={next}>{t("onboarding.skip")}</button>}
          {step === "asr" && asr?.primary}
          {step === "llm" && llm?.primary}
          {(step === "language" || step === "permission") && <button type="button" className="btn btn-primary btn-sm" onClick={next}>{t("onboarding.next")}</button>}
          {step === "done" && <button type="button" className="btn btn-primary btn-sm" onClick={() => api.finishOnboarding().catch(setError)}>{t("onboarding.finish")}</button>}
        </div>
      </div>
    </div>
  );
}
