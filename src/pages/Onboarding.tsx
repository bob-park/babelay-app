import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ModelRow } from "../components/ModelRow";
import { PillButton } from "../components/PillButton";
import { ASR_MODELS, BALANCED, LLM_MODELS } from "../lib/models.fixture";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";
import type { UiLang } from "../lib/types";

type Step = "language" | "permission" | "asr" | "llm" | "done";

export default function Onboarding() {
  const { t } = useTranslation();
  const { settings, update } = useSettings();
  const [steps, setSteps] = useState<Step[]>(["language", "permission", "asr", "llm", "done"]);
  const [idx, setIdx] = useState(0);
  const [perm, setPerm] = useState<"granted" | "denied" | "unknown" | null>(null);

  useEffect(() => {
    api.getPlatform().then((p) => { if (p !== "macos") setSteps(["language", "asr", "llm", "done"]); });
  }, []);

  if (!settings) return null;
  const step = steps[idx];
  const next = () => setIdx((i) => Math.min(i + 1, steps.length - 1));
  const back = () => setIdx((i) => Math.max(i - 1, 0));

  const stepLabel: Record<Step, string> = {
    language: t("onboarding.stepLanguage"), permission: t("onboarding.stepPermission"),
    asr: t("onboarding.stepAsr"), llm: t("onboarding.stepLlm"), done: t("onboarding.stepDone"),
  };
  const langBtn = (v: UiLang, text: string) => (
    <PillButton key={v} variant={settings.general.ui_language === v ? "primary" : "default"} onClick={() => update({ general: { ui_language: v } })}>{text}</PillButton>
  );

  return (
    <div className="flex h-full flex-col p-6">
      <div className="mb-4 flex gap-4 text-[11px] font-bold uppercase tracking-[1.2px]">
        {steps.map((s, i) => (
          <span key={s} className={`flex items-center gap-1 ${i > idx ? "text-fg-muted" : "text-fg"}`}>
            {i < idx ? <span className="rounded-full bg-accent px-1.5 text-accent-fg">✓</span> : `${i + 1} `}
            {stepLabel[s]}
          </span>
        ))}
      </div>

      <div className="flex flex-1 flex-col gap-3 overflow-auto">
        {step === "language" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.languageTitle")}</h2>
            <div className="flex flex-wrap gap-2">
              {langBtn("system", t("general.langSystem"))}{langBtn("ko", "한국어")}{langBtn("en", "English")}{langBtn("ja", "日本語")}
            </div>
          </>
        )}
        {step === "permission" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.permissionTitle")}</h2>
            <p className="text-fg-muted">{t("onboarding.permissionDesc")}</p>
            <div className="flex gap-2">
              <PillButton variant="primary" onClick={() => api.checkAudioPermission().then(setPerm)}>{t("onboarding.permissionCheck")}</PillButton>
              <PillButton variant="outline" onClick={() => api.openPrivacySettings()}>{t("onboarding.openSettings")}</PillButton>
            </div>
            {perm && <p className="text-sm">{t(`onboarding.permission${perm[0].toUpperCase()}${perm.slice(1)}`)}</p>}
          </>
        )}
        {step === "asr" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.asrTitle")}</h2>
            <p className="text-fg-muted">{t("onboarding.asrDesc")}</p>
            {ASR_MODELS.map((m) => (
              <ModelRow key={m.id} model={m} selected={settings.asr.model_id === m.id} badges={{ balanced: m.id === BALANCED.asr }} onSelect={() => update({ asr: { model_id: m.id } })} />
            ))}
          </>
        )}
        {step === "llm" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.llmTitle")}</h2>
            <p className="text-fg-muted">{t("onboarding.llmDesc")}</p>
            {LLM_MODELS.map((m) => (
              <ModelRow key={m.id} model={m} selected={settings.translation.local_model === m.id} badges={{ balanced: m.id === BALANCED.llm }} onSelect={() => update({ translation: { local_model: m.id } })} />
            ))}
          </>
        )}
        {step === "done" && (
          <>
            <h2 className="text-2xl font-bold">{t("onboarding.doneTitle")}</h2>
            <p className="text-fg-muted">{t("onboarding.doneDesc")}</p>
          </>
        )}
      </div>

      <div className="mt-4 flex items-center justify-between">
        <PillButton onClick={back} disabled={idx === 0}>{t("onboarding.back")}</PillButton>
        <div className="flex gap-2">
          {step === "llm" && <PillButton variant="outline" onClick={next}>{t("onboarding.skip")}</PillButton>}
          {step === "done"
            ? <PillButton variant="primary" onClick={() => api.finishOnboarding()}>{t("onboarding.finish")}</PillButton>
            : <PillButton variant="primary" onClick={next}>{t("onboarding.next")}</PillButton>}
        </div>
      </div>
    </div>
  );
}
