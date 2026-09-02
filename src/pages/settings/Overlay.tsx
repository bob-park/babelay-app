import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { PillButton } from "../../components/PillButton";
import { api } from "../../lib/tauri";
import { useSettings } from "../../lib/settings";
import type { DisplayMode, SourceLang, UiLang } from "../../lib/types";

const label = "text-[10px] font-bold uppercase tracking-[1.2px] text-fg-muted";
const select = "rounded-md bg-surface px-3 py-2 text-sm text-fg";

export default function OverlaySettings() {
  const { t } = useTranslation();
  const { settings, update, setError } = useSettings();
  const [adjust, setAdjust] = useState(false);

  // 트레이·단축키로도 조정 모드가 꺼지므로 백엔드가 진실이다.
  useEffect(() => {
    const un = listen<boolean>("overlay-adjust-mode", (e) => setAdjust(e.payload));
    return () => { un.then((f) => f()); };
  }, []);
  // 페이지를 떠날 때는 무조건 끈다(이미 꺼져 있으면 무해).
  useEffect(() => () => { api.overlaySetAdjustMode(false).catch(setError); }, []);

  if (!settings) return null;
  const o = settings.overlay;

  const toggleAdjust = () => {
    const next = !adjust;
    setAdjust(next); // 낙관적. 백엔드 이벤트가 곧 확정한다.
    api.overlaySetAdjustMode(next).catch(setError);
  };

  const modes: DisplayMode[] = ["both", "source", "target"];
  const modeLabel = { both: t("overlay.modeBoth"), source: t("overlay.modeSource"), target: t("overlay.modeTarget") };

  return (
    <div className="flex max-w-xl flex-col gap-6">
      <h2 className="text-2xl font-bold">{t("settings.overlay")}</h2>

      <div className="flex items-center justify-between">
        <span className={label}>{t("overlay.adjust")}</span>
        <PillButton variant={adjust ? "primary" : "default"} onClick={toggleAdjust}>
          {adjust ? t("overlay.adjustOn") : t("overlay.adjustOff")}
        </PillButton>
      </div>

      <div className="flex flex-col gap-2">
        <span className={label}>{t("overlay.displayMode")}</span>
        <div className="flex gap-2">
          {modes.map((m) => (
            <PillButton key={m} size="sm" variant={o.display_mode === m ? "primary" : "default"} onClick={() => update({ overlay: { display_mode: m } })}>
              {modeLabel[m]}
            </PillButton>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="flex flex-col gap-1">
          <span className={label}>{t("overlay.subtitleLang")}</span>
          <select className={select} value={o.subtitle_lang} onChange={(e) => update({ overlay: { subtitle_lang: e.target.value as UiLang } })}>
            <option value="system">{t("general.langSystem")}</option>
            <option value="ko">{t("general.langKo")}</option>
            <option value="en">{t("general.langEn")}</option>
            <option value="ja">{t("general.langJa")}</option>
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <span className={label}>{t("overlay.sourceLang")}</span>
          <select className={select} value={settings.asr.source_lang} onChange={(e) => update({ asr: { source_lang: e.target.value as SourceLang } })}>
            <option value="auto">{t("overlay.auto")}</option>
            <option value="ko">{t("general.langKo")}</option>
            <option value="en">{t("general.langEn")}</option>
            <option value="ja">{t("general.langJa")}</option>
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <span className={label}>{t("overlay.fontSize")} · {o.font_size}px</span>
          <input type="range" min={14} max={64} value={o.font_size} onChange={(e) => update({ overlay: { font_size: Number(e.target.value) } })} />
        </div>
        <div className="flex flex-col gap-1">
          <span className={label}>{t("overlay.bgOpacity")} · {Math.round(o.bg_opacity * 100)}%</span>
          <input type="range" min={0} max={100} value={Math.round(o.bg_opacity * 100)} onChange={(e) => update({ overlay: { bg_opacity: Number(e.target.value) / 100 } })} />
        </div>
      </div>
    </div>
  );
}
