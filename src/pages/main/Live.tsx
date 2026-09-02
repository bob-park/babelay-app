import { useTranslation } from "react-i18next";
import { PillButton } from "../../components/PillButton";
import { useModels } from "../../lib/models";
import { useSession } from "../../lib/session";
import { useSettings } from "../../lib/settings";

export default function Live() {
  const { t } = useTranslation();
  const { capturing, toggle } = useSession();
  const { settings, update } = useSettings();
  const models = useModels((s) => s.models);
  if (!settings) return null;
  const name = (id: string) => models.find((m) => m.info.id === id)?.info.name ?? id;
  const src = settings.asr.source_lang === "auto" ? t("overlay.auto") : settings.asr.source_lang.toUpperCase();
  const tgt = settings.overlay.subtitle_lang === "system" ? t("general.langSystem") : settings.overlay.subtitle_lang.toUpperCase();

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">{t("nav.live")}</h2>
        <div className="flex gap-2">
          <PillButton variant="primary" onClick={toggle}>{capturing ? `■ ${t("live.stop")}` : `● ${t("live.start")}`}</PillButton>
          <PillButton variant={settings.overlay.enabled ? "default" : "ghost"} onClick={() => update({ overlay: { enabled: !settings.overlay.enabled } })}>{t("live.overlay")}</PillButton>
        </div>
      </div>
      <div className="text-xs text-fg-muted">{src} → {tgt} · {name(settings.asr.model_id)}{settings.translation.backend === "local" ? ` · ${name(settings.translation.local_model)}` : ""}</div>
      <div className="flex-1 rounded-[var(--radius-card)] bg-base-2" />
    </div>
  );
}
