import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { ToastContainer, toast } from "react-toastify";
import { formatSize, useModels } from "../lib/models";
import { useUpdate } from "../lib/update";

const DOWNLOAD = "download";
const QUEUE = "download-queue:";
const UPDATE_AVAILABLE = "update-available";
const UPDATE_INSTALL = "update-install";

// 받는 중인 모델 하나. 토스트 본문은 스토어를 직접 구독하므로 toast.update 로 다시 그릴 필요가 없다.
function DownloadBody() {
  const { t } = useTranslation();
  const active = useModels((s) => s.models.find((m) => m.download));
  const cancel = useModels((s) => s.cancel);
  if (!active?.download) return null;
  return (
    <div className="relative w-full pr-7 text-sm">
      {/* 취소는 오른쪽 위에 고정. 제목이 길어져도 버튼 아래로 들어가지 않게 본문에 오른쪽 여백. */}
      <button type="button" className="btn btn-ghost btn-xs absolute right-0 top-0" aria-label={t("models.cancel")} onClick={() => cancel(active.info.id)}>✕</button>
      <div className="truncate font-semibold">{t("downloads.downloading", { name: active.info.name })}</div>
      <div className="text-xs opacity-70">
        {Math.round((active.download.received / Math.max(1, active.download.total)) * 100)}% · {formatSize(active.download.received)} / {formatSize(active.download.total)}
      </div>
    </div>
  );
}

// 대기 중인 모델 하나. 진행 토스트 뒤에 겹쳐 쌓인다(ToastContainer stacked).
function QueueBody({ id }: { id: string }) {
  const { t } = useTranslation();
  const name = useModels((s) => s.models.find((m) => m.info.id === id)?.info.name ?? id);
  const dequeue = useModels((s) => s.dequeue);
  return (
    <div className="relative w-full pr-7 text-sm">
      <button type="button" className="btn btn-ghost btn-xs absolute right-0 top-0" aria-label={t("models.cancel")} onClick={() => dequeue(id)}>✕</button>
      <div className="truncate opacity-70">{t("downloads.next", { name })}</div>
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
  const downloading = useModels((s) => s.models.some((m) => m.download));
  const queue = useModels((s) => s.queue);
  const received = useModels((s) => s.models.find((m) => m.download)?.download?.received ?? 0);
  const total = useModels((s) => s.models.find((m) => m.download)?.download?.total ?? 0);
  const info = useUpdate((s) => s.info);
  const progress = useUpdate((s) => s.progress);

  // stacked 모드는 최근 토스트가 앞에 온다. 대기 항목이 생길 때마다 진행 토스트를 새 id 로 다시 띄워
  // 항상 맨 앞에 두고, 대기 항목들은 그 뒤에 겹치게 한다.
  const dlId = useRef<string | null>(null);
  const dlProgress = total > 0 ? Math.min(received / total, 0.999) : 0;
  const showDownload = (p: number) => {
    if (dlId.current) toast.dismiss(dlId.current);
    dlId.current = `${DOWNLOAD}:${Date.now()}`;
    toast(<DownloadBody />, { toastId: dlId.current, autoClose: false, closeButton: false, closeOnClick: false, draggable: false, progress: p });
  };
  useEffect(() => {
    if (!downloading) {
      if (dlId.current) { toast.dismiss(dlId.current); dlId.current = null; }
      return;
    }
    // 진행률 1 은 "끝났다"로 읽혀 토스트가 닫힌다. 완료는 downloading 이 꺼질 때 닫는다.
    if (dlId.current && toast.isActive(dlId.current)) toast.update(dlId.current, { progress: dlProgress });
    else showDownload(dlProgress);
  }, [downloading, dlProgress]);

  // 대기열은 항목마다 토스트 하나. 빠진 항목은 닫고, 새 항목만 띄운다.
  const shownQueue = useRef<string[]>([]);
  useEffect(() => {
    for (const id of shownQueue.current) if (!queue.includes(id)) toast.dismiss(QUEUE + id);
    let added = false;
    for (const id of queue) {
      if (toast.isActive(QUEUE + id)) continue;
      toast(<QueueBody id={id} />, { toastId: QUEUE + id, autoClose: false, closeButton: false, closeOnClick: false, draggable: false, hideProgressBar: true });
      added = true;
    }
    shownQueue.current = queue;
    if (added && downloading) showDownload(dlProgress);
  }, [queue]);

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

  return <ToastContainer position="top-right" theme="light" stacked closeOnClick pauseOnFocusLoss={false} />;
}
