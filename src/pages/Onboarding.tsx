import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../components/icons";
import { ModelPicker } from "../components/ModelPicker";
import { PermissionIcon, PermissionRow, type Perm } from "../components/PermissionRow";
import { SettingGroup } from "../components/SettingGroup";
import { useModels } from "../lib/models";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";
import { showError } from "../lib/toast";
import type { UiLang } from "../lib/types";

type Step = "language" | "permission" | "models" | "done";
const ALL: Step[] = ["language", "permission", "models", "done"];

export default function Onboarding() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const { models, refresh, queue } = useModels();
  const [steps, setSteps] = useState<Step[]>(ALL);
  const [idx, setIdx] = useState(0);
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
  const langBtn = (v: UiLang, text: string) => (
    <button key={v} type="button" className={`btn btn-sm ${settings.general.ui_language === v ? "btn-primary" : "border-base-300"}`} onClick={() => update({ general: { ui_language: v } })}>{text}</button>
  );
  const pct = (m: typeof asr) => (m?.download ? `${Math.round((m.download.received / Math.max(1, m.download.total)) * 100)}%` : null);
  // 대기열에 있는 모델은 아직 실패가 아니다. ✕ 대신 중립 표시.
  const mark = (m: typeof asr) => m?.installed
    ? <span role="img" aria-label={t("models.badgeInstalled")} className="flex h-6 w-6 items-center justify-center rounded-full bg-success text-success-content"><Icon name="check" /></span>
    : m?.download
      ? <span className="text-xs tabular-nums text-fg-muted">{pct(m)}</span>
      : m && queue.includes(m.info.id)
        ? <span role="img" aria-label={t("downloads.next", { name: m.info.name })} className="flex h-6 w-6 items-center justify-center rounded-full bg-base-300 text-base-content"><Icon name="help" /></span>
        : <span role="img" aria-label={t("models.notInstalled")} className="flex h-6 w-6 items-center justify-center rounded-full bg-error text-error-content"><Icon name="x" /></span>;

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <ul className="steps w-full text-xs">
        {steps.map((s, i) => (
          <li key={s} className={`step ${i <= cur ? "step-primary" : ""}`} data-content={i < cur ? "✓" : String(i + 1)}>{t(`onboarding.step.${s}`)}</li>
        ))}
      </ul>
      <div className="mx-auto flex w-full max-w-2xl flex-1 flex-col gap-4 overflow-hidden">
        <h2 className="text-2xl font-bold tracking-tight">{t(`onboarding.title.${step}`)}</h2>

        <div className="flex flex-1 flex-col gap-2 overflow-auto">
          {step === "language" && <div className="flex flex-wrap gap-2">{langBtn("system", t("general.langSystem"))}{langBtn("ko", "한국어")}{langBtn("en", "English")}{langBtn("ja", "日本語")}</div>}
          {step === "permission" && <PermissionRow onStatus={setPerm} />}
          {step === "models" && <ModelPicker applyLabel={t("onboarding.next")} onApplied={next} />}
          {step === "done" && (
            <SettingGroup>
              <div className="flex items-center justify-between px-4 py-3 text-sm"><span>{t("onboarding.check.asr")} · {asr?.info.name ?? "—"}</span>{mark(asr)}</div>
              <div className="flex items-center justify-between px-4 py-3 text-sm"><span>{t("onboarding.check.llm")} · {llm?.info.name ?? "—"}</span>{mark(llm)}</div>
              {macos && <div className="flex items-center justify-between px-4 py-3 text-sm"><span>{t("permission.name")}</span><PermissionIcon perm={perm} /></div>}
            </SettingGroup>
          )}
        </div>

        <div className="flex items-center justify-between">
          <button type="button" className="btn btn-ghost btn-sm" onClick={back} disabled={cur === 0}>{t("onboarding.back")}</button>
          <div className="flex gap-2">
            {(step === "language" || step === "permission") && <button type="button" className="btn btn-primary btn-sm" onClick={next}>{t("onboarding.next")}</button>}
            {step === "done" && <button type="button" className="btn btn-primary btn-sm" disabled={!asr?.installed} onClick={() => api.finishOnboarding().catch(showError)}>{t("onboarding.finish")}</button>}
          </div>
        </div>
      </div>
    </div>
  );
}
