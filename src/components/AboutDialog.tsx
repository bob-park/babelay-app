import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../lib/tauri";
import type { HwInfo } from "../lib/types";
import logo from "../../assets/icon.svg";

// "정보" 버튼 + macOS "이 Mac에 관하여" 식 대화상자. 설정 › 일반에서만 쓴다.
export function AboutDialog() {
  const { t } = useTranslation();
  const ref = useRef<HTMLDialogElement>(null);
  const [hw, setHw] = useState<HwInfo | null>(null);
  // 설정 페이지에 들어올 때 한 번만 조회한다. 열 때마다 IPC 를 부르지 않는다.
  useEffect(() => { api.getHwInfo().then(setHw).catch(() => {}); }, []);

  return (
    <>
      <button type="button" className="btn btn-sm border-base-300" onClick={() => ref.current?.showModal()}>{t("nav.about")}</button>
      <dialog ref={ref} className="modal">
        <div className="modal-box max-w-xs shadow-kr">
          <div className="flex flex-col items-center gap-1 text-center">
            <img src={logo} alt="" className="h-16 w-16" />
            <h3 className="mt-2 text-lg font-bold tracking-tight">{t("app.name")}</h3>
            <div className="text-xs text-fg-muted">{t("about.version")} {import.meta.env.PACKAGE_VERSION}</div>
          </div>
          {hw && (
            <dl className="mx-auto mt-5 grid w-fit grid-cols-[auto_auto] gap-x-4 gap-y-1 text-sm">
              <dt className="text-right text-fg-muted">{t("about.chip")}</dt><dd>{hw.chip}</dd>
              <dt className="text-right text-fg-muted">{t("about.memory")}</dt><dd>{hw.mem_gb} GB</dd>
              {hw.gpu && <><dt className="text-right text-fg-muted">{t("about.graphics")}</dt><dd>{hw.gpu}{hw.gpu_mem_gb ? ` · ${hw.gpu_mem_gb} GB` : ""}</dd></>}
            </dl>
          )}
          <div className="modal-action justify-center">
            <form method="dialog"><button className="btn btn-sm border-base-300">{t("common.close")}</button></form>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop"><button aria-label={t("common.close")} /></form>
      </dialog>
    </>
  );
}
