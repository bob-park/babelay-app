import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { SessionBadges } from "../../components/SessionBadges";
import { translatorLabel, useModels } from "../../lib/models";
import { clock, useSession } from "../../lib/session";
import { useSettings } from "../../lib/settings";
import type { SourceLang, UiLang } from "../../lib/types";

export default function Live() {
  const { t } = useTranslation();
  const { view } = useSession();
  const { settings, update } = useSettings();
  const models = useModels((s) => s.models);
  const end = useRef<HTMLDivElement>(null);

  // 새 줄이 들어오면 바닥에 붙인다. 안 그러면 말이 화면 밖에서 흐른다.
  useEffect(() => { end.current?.scrollIntoView({ block: "end" }); }, [view.finals[view.finals.length - 1]?.id, view.partial?.text]);

  if (!settings) return null;
  const name = (id: string) => (id ? models.find((m) => m.info.id === id)?.info.name ?? id : "—");
  // 원문만 모드는 번역이 없으니 타겟 select 와 번역 모델 배지를 뺀다.
  const translating = settings.overlay.display_mode !== "source";
  const tr = settings.translation;
  const translator = translating ? translatorLabel(tr.backend === "local" ? `local:${tr.local_model}` : `cloud:${tr.cloud.provider}/${tr.cloud.model}`, name) : null;
  const badge = "badge badge-ghost badge-sm";

  return (
    <div className="flex h-full flex-col gap-3">
      {/* 컨트롤 줄: 언어 쌍(설정값, 다음 세션부터 적용) · 상태 점 · 모델 배지 · 오버레이 토글 */}
      <div className="flex flex-wrap items-center gap-2 text-xs text-fg-muted">
        <select className="select select-sm w-44" aria-label={t("translation.sourceLang")} value={settings.asr.source_lang} onChange={(e) => update({ asr: { source_lang: e.target.value as SourceLang } })}>
          <option value="auto">{t("translation.auto")}</option><option value="ko">{t("general.langKo")}</option><option value="en">{t("general.langEn")}</option><option value="ja">{t("general.langJa")}</option>
        </select>
        {translating && (
          <>
            <span aria-hidden="true">→</span>
            <select className="select select-sm w-44" aria-label={t("translation.targetLang")} value={settings.overlay.subtitle_lang} onChange={(e) => update({ overlay: { subtitle_lang: e.target.value as UiLang } })}>
              <option value="system">{t("general.langSystem")}</option><option value="ko">{t("general.langKo")}</option><option value="en">{t("general.langEn")}</option><option value="ja">{t("general.langJa")}</option>
            </select>
          </>
        )}
        <span className={`ml-1 h-2 w-2 shrink-0 rounded-full ${view.capturing ? "bg-success" : "bg-fg-muted"}`} />
        {/* 캡처 중에는 돌고 있는 세션의 값(언어 포함)을, 정지 중에는 설정된 모델만 보여준다. */}
        {view.capturing
          ? <SessionBadges src={view.sourceLang ?? "auto"} tgt={translating ? view.targetLang : null} asrModel={view.modelId ?? settings.asr.model_id} translator={translator} />
          : <><span className={badge}>{name(settings.asr.model_id)}</span>{translator && <span className={badge}>{translator}</span>}</>}
        {view.gpuFallback && <span className="badge badge-warning badge-sm">{t("live.cpuFallback")}</span>}
        {view.lagging && <span className="badge badge-warning badge-sm">{t("live.lagging")}</span>}
        <label className="ml-auto flex items-center gap-2 text-sm text-base-content">
          {t("live.overlay")}
          <input type="checkbox" role="switch" className="toggle toggle-primary toggle-sm" checked={settings.overlay.enabled} onChange={(e) => update({ overlay: { enabled: e.target.checked } })} />
        </label>
      </div>

      <div className="flex-1 overflow-auto rounded-box border border-base-300 bg-base-100 p-4 shadow-kr">
        <div className="flex flex-col gap-2 text-sm">
          {view.finals.map((f) => (
            <div key={f.id} className="flex gap-3">
              <span className="shrink-0 tabular-nums text-fg-muted">{clock(f.start_ms)}</span>
              <div className="min-w-0 break-words">
                <div className="text-fg-muted">{f.text}</div>
                {f.tgt && <div className="font-semibold">{f.tgt}</div>}
              </div>
            </div>
          ))}
          {view.partial && (
            <div className="flex gap-3 text-fg-muted opacity-70">
              <span className="shrink-0 tabular-nums">{clock(view.partial.start_ms)}</span>
              <span className="min-w-0 break-words">{view.partial.text}</span>
            </div>
          )}
          <div ref={end} />
        </div>
      </div>
    </div>
  );
}
