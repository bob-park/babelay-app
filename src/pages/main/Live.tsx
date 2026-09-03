import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useModels } from "../../lib/models";
import { clock, useSession } from "../../lib/session";
import { useSettings } from "../../lib/settings";

export default function Live() {
  const { t } = useTranslation();
  const { view, start, stop } = useSession();
  const { settings, update } = useSettings();
  const models = useModels((s) => s.models);
  const end = useRef<HTMLDivElement>(null);

  // 새 줄이 들어오면 바닥에 붙인다. 안 그러면 말이 화면 밖에서 흐른다.
  useEffect(() => { end.current?.scrollIntoView({ block: "end" }); }, [view.finals[view.finals.length - 1]?.id, view.partial?.text]);

  if (!settings) return null;
  const name = (id: string) => models.find((m) => m.info.id === id)?.info.name ?? id;
  // 캡처 중에는 돌고 있는 세션의 설정을 보여준다 — 설정을 바꿔도 다음 세션부터 적용된다.
  const srcLang = (view.capturing ? view.sourceLang ?? "auto" : settings.asr.source_lang);
  const asrModel = (view.capturing ? view.modelId : null) ?? settings.asr.model_id;
  const src = srcLang === "auto" ? t("translation.auto") : srcLang.toUpperCase();
  const tgt = settings.overlay.subtitle_lang === "system" ? t("general.langSystem") : settings.overlay.subtitle_lang.toUpperCase();

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">{t("nav.live")}</h2>
        <div className="flex gap-2">
          <button type="button" className="btn btn-primary btn-sm" disabled={view.stopping} onClick={() => (view.capturing ? stop() : start())}>
            {view.stopping ? t("live.stopping") : view.capturing ? `■ ${t("live.stop")}` : `● ${t("live.start")}`}
          </button>
          <button type="button" className={`btn btn-sm ${settings.overlay.enabled ? "btn-neutral" : "btn-ghost"}`} onClick={() => update({ overlay: { enabled: !settings.overlay.enabled } })}>{t("live.overlay")}</button>
        </div>
      </div>

      <div className="flex items-center gap-2 text-xs text-fg-muted">
        <span className={`h-2 w-2 shrink-0 rounded-full ${view.capturing ? "bg-primary" : "bg-fg-muted"}`} />
        <span>{src} → {tgt} · {name(asrModel)}{settings.translation.backend === "local" ? ` · ${name(settings.translation.local_model)}` : ""}</span>
        {view.gpuFallback && <span className="badge badge-neutral badge-sm">{t("live.cpuFallback")}</span>}
        {view.lagging && <span className="badge badge-neutral badge-sm">{t("live.lagging")}</span>}
      </div>

      <div className="flex-1 overflow-auto rounded-box bg-base-200 p-4">
        <div className="flex flex-col gap-2 text-sm">
          {view.finals.map((f) => (
            <div key={f.id} className="flex gap-3">
              <span className="shrink-0 tabular-nums text-fg-muted">{clock(f.start_ms)}</span>
              <span className="min-w-0 break-words">{f.text}</span>
            </div>
          ))}
          {view.partial && (
            <div className="flex gap-3 text-fg-muted">
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
