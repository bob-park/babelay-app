import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ToastContainer, toast } from "react-toastify";
import { formatSize, useModels } from "../lib/models";
import { useUpdate } from "../lib/update";

const DOWNLOAD = "download";
const UPDATE_AVAILABLE = "update-available";
const UPDATE_INSTALL = "update-install";

// 받는 중인 모델 하나 + 대기 목록. 토스트 본문은 스토어를 직접 구독하므로 toast.update 로 다시 그릴 필요가 없다.
function DownloadBody() {
  const { t } = useTranslation();
  const models = useModels((s) => s.models);
  const queue = useModels((s) => s.queue);
  const cancel = useModels((s) => s.cancel);
  const dequeue = useModels((s) => s.dequeue);
  const active = models.find((m) => m.download);
  const name = (id: string) => models.find((m) => m.info.id === id)?.info.name ?? id;
  return (
    <div className="relative pr-7 text-sm">
      {/* 취소는 오른쪽 위에 고정. 제목이 길어져도 버튼 아래로 들어가지 않게 본문에 오른쪽 여백. */}
      {active?.download && (
        <button type="button" className="btn btn-ghost btn-xs absolute -right-1 -top-1" aria-label={t("models.cancel")} onClick={() => cancel(active.info.id)}>✕</button>
      )}
      {active?.download && (
        <div className="truncate font-semibold">{t("downloads.downloading", { name: active.info.name })}</div>
      )}
      {active?.download && (
        <div className="text-xs opacity-70">
          {Math.round((active.download.received / Math.max(1, active.download.total)) * 100)}% · {formatSize(active.download.received)} / {formatSize(active.download.total)}
        </div>
      )}
      {queue.map((id) => (
        <div key={id} className="mt-1 flex items-center justify-between gap-2 text-xs opacity-70">
          <span className="truncate">{t("downloads.next", { name: name(id) })}</span>
          <button type="button" className="btn btn-ghost btn-xs" aria-label={t("models.cancel")} onClick={() => dequeue(id)}>✕</button>
        </div>
      ))}
    </div>
  );
}

function UpdateAvailableBody({ version, closeToast }: { version: string; closeToast?: () => void }) {
  const { t } = useTranslation();
  const install = useUpdate((s) => s.install);
  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <span>{t("update.available", { version })}</span>
      <button type="button" className="btn btn-primary btn-xs" onClick={() => { closeToast?.(); install(); }}>{t("update.install")}</button>
    </div>
  );
}

// 스토어 상태를 토스트로 비춘다. 스토어는 토스트를 모른다.
export function Toasts() {
  const { t } = useTranslation();
  const downloading = useModels((s) => s.models.some((m) => m.download) || s.queue.length > 0);
  const received = useModels((s) => s.models.find((m) => m.download)?.download?.received ?? 0);
  const total = useModels((s) => s.models.find((m) => m.download)?.download?.total ?? 0);
  const info = useUpdate((s) => s.info);
  const progress = useUpdate((s) => s.progress);

  useEffect(() => {
    if (!downloading) { toast.dismiss(DOWNLOAD); return; }
    // 진행률 1 은 "끝났다"로 읽혀 토스트가 닫힌다. 완료는 downloading 이 꺼질 때 닫는다.
    const p = total > 0 ? Math.min(received / total, 0.999) : 0;
    if (toast.isActive(DOWNLOAD)) toast.update(DOWNLOAD, { progress: p });
    else toast(<DownloadBody />, { toastId: DOWNLOAD, autoClose: false, closeButton: false, closeOnClick: false, draggable: false, progress: p });
  }, [downloading, received, total]);

  useEffect(() => {
    if (!info) { toast.dismiss(UPDATE_AVAILABLE); return; }
    if (!toast.isActive(UPDATE_AVAILABLE)) {
      toast.info(({ closeToast }) => <UpdateAvailableBody version={info.version} closeToast={closeToast} />, { toastId: UPDATE_AVAILABLE, autoClose: false, closeOnClick: false });
    }
  }, [info]);

  useEffect(() => {
    if (!progress) { toast.dismiss(UPDATE_INSTALL); return; }
    const p = progress.total ? Math.min(progress.received / progress.total, 0.999) : 0;
    if (toast.isActive(UPDATE_INSTALL)) toast.update(UPDATE_INSTALL, { progress: p });
    else toast(t("update.installing"), { toastId: UPDATE_INSTALL, autoClose: false, closeButton: false, closeOnClick: false, draggable: false, progress: p });
  }, [progress, t]);

  return <ToastContainer position="top-right" theme="light" newestOnTop closeOnClick pauseOnFocusLoss={false} />;
}
