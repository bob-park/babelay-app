import { useTranslation } from "react-i18next";
import { PillButton } from "../../components/PillButton";
import { useSession } from "../../lib/session";
import { useSettings } from "../../lib/settings";

export default function Live() {
  const { t } = useTranslation();
  const { capturing, toggle } = useSession();
  const { settings, update } = useSettings();
  if (!settings) return null;
  const overlayOn = settings.overlay.enabled;

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <div className="flex gap-2">
          <PillButton variant="primary" onClick={toggle}>{capturing ? `■ ${t("live.stop")}` : `▶ ${t("live.start")}`}</PillButton>
          <PillButton onClick={() => update({ overlay: { enabled: !overlayOn } })}>
            {overlayOn ? t("live.overlayOn") : t("live.overlayOff")}
          </PillButton>
        </div>
        <span className="text-xs text-fg-muted">
          {settings.asr.source_lang.toUpperCase()} → {settings.overlay.subtitle_lang.toUpperCase()} · {settings.asr.model_id}
        </span>
      </div>
      <div className="flex-1 rounded-lg bg-base-2 p-4 text-sm text-fg-muted">
        {t("live.empty")}
        <div className="mt-2 text-xs">{t("live.shortcutHint", { capture: "⌘/Ctrl+Shift+S", overlay: "⌘/Ctrl+Shift+O" })}</div>
      </div>
    </div>
  );
}
