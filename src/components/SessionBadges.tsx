import { useTranslation } from "react-i18next";
import { langName } from "../lib/i18n";
import { useModels } from "../lib/models";
import type { HwInfo } from "../lib/types";

/** 장비 스펙 배지: 칩, 메모리, GPU(+VRAM). */
export function HwBadges({ hw }: { hw: HwInfo }) {
  const cls = "badge badge-ghost badge-sm";
  return (
    <>
      <span className={cls}>{hw.chip}</span>
      <span className={cls}>{hw.mem_gb} GB</span>
      {hw.gpu && <span className={cls}>{hw.gpu}{hw.gpu_mem_gb ? ` · ${hw.gpu_mem_gb} GB` : ""}</span>}
    </>
  );
}

/** 세션 요약 배지: 원어 [→ 타겟] 전사 모델 [번역기]. 타겟과 번역기는 없으면 뺀다. */
export function SessionBadges({ src, tgt, asrModel, translator }: { src: string; tgt: string | null; asrModel: string; translator: string | null }) {
  const { t, i18n } = useTranslation();
  const models = useModels((s) => s.models);
  const name = (id: string) => (id ? models.find((m) => m.info.id === id)?.info.name ?? id : "—");
  const lang = (code: string) => (code === "auto" ? t("translation.auto") : code === "system" ? t("general.langSystem") : langName(code, t, i18n.language));
  const cls = "badge badge-ghost badge-sm";
  return (
    <>
      <span className={cls}>{lang(src)}</span>
      {tgt && <><span aria-hidden="true">→</span><span className={cls}>{lang(tgt)}</span></>}
      <span className={cls}>{name(asrModel)}</span>
      {translator && <span className={cls}>{translator}</span>}
    </>
  );
}
