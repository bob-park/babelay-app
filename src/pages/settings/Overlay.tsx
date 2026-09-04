import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { SegmentedControl } from "../../components/SegmentedControl";
import { SettingGroup, SettingRow } from "../../components/SettingGroup";
import { api } from "../../lib/tauri";
import { useSettings } from "../../lib/settings";
import { overlayLines } from "../../lib/overlay";
import type { DisplayMode } from "../../lib/types";

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
  const toggleAdjust = () => { const next = !adjust; setAdjust(next); api.overlaySetAdjustMode(next).catch((e) => { setAdjust(!next); setError(e); }); };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <span className="text-sm text-fg-muted">{t("overlay.preview")}</span>
        <button type="button" className={`btn btn-sm ${adjust ? "btn-primary" : "btn-neutral"}`} onClick={toggleAdjust}>{adjust ? t("overlay.adjustDone") : t("overlay.adjust")}</button>
      </div>

      {(() => {
        const lines = overlayLines(o.display_mode, t("overlay.previewSource"), "", t("overlay.previewTarget"));
        const scale = o.font_size / 28;
        return (
          <div className="@container relative aspect-[16/7] w-full rounded-box bg-[linear-gradient(#1b2230,#0e1218)]">
            <div className="absolute bottom-[6%] left-1/2 w-[60%] -translate-x-1/2 rounded-[1cqw] px-[3cqw] py-[2.5cqw] text-center text-white" style={{ background: `rgba(18,18,18,${o.bg_opacity})` }}>
              {lines.map((l, i) => (
                l.muted
                  ? <div key={i} className="text-neutral-400" style={{ fontSize: `${2.6 * scale}cqw` }}>{l.text}</div>
                  : <div key={i} className="font-bold leading-tight" style={{ fontSize: `${4.2 * scale}cqw` }}>{l.text}</div>
              ))}
            </div>
          </div>
        );
      })()}

      <SettingGroup>
        <SettingRow as="div" label={t("overlay.displayMode")}>
          <SegmentedControl value={o.display_mode} onChange={(v: DisplayMode) => update({ overlay: { display_mode: v } })}
            options={[{ value: "both", label: t("overlay.modeBoth") }, { value: "source", label: t("overlay.modeSource") }, { value: "target", label: t("overlay.modeTarget") }]} />
        </SettingRow>
      </SettingGroup>

      <SettingGroup>
        <SettingRow label={t("overlay.fontSize")}>
          <input type="range" min={14} max={64} value={o.font_size} onChange={(e) => update({ overlay: { font_size: Number(e.target.value) } })} className="range range-primary range-xs w-40" />
          <span className="w-8 text-right tabular-nums">{o.font_size}</span>
        </SettingRow>
        <SettingRow label={t("overlay.bgOpacity")}>
          <input type="range" min={0} max={100} value={Math.round(o.bg_opacity * 100)} onChange={(e) => update({ overlay: { bg_opacity: Number(e.target.value) / 100 } })} className="range range-primary range-xs w-40" />
          <span className="w-8 text-right tabular-nums">{Math.round(o.bg_opacity * 100)}%</span>
        </SettingRow>
      </SettingGroup>
    </div>
  );
}
