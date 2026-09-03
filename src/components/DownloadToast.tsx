import { useTranslation } from "react-i18next";
import { formatSize, useModels } from "../lib/models";

// 오른쪽 상단. 받는 중인 모델 카드 하나 + 대기 목록. 아무것도 없으면 그리지 않는다.
export function DownloadToast() {
  const { t } = useTranslation();
  const models = useModels((s) => s.models);
  const queue = useModels((s) => s.queue);
  const cancel = useModels((s) => s.cancel);
  const dequeue = useModels((s) => s.dequeue);
  const active = models.find((m) => m.download);
  if (!active?.download && queue.length === 0) return null;
  const name = (id: string) => models.find((m) => m.info.id === id)?.info.name ?? id;

  return (
    <div className="toast toast-top toast-end z-50">
      <div className="w-64 rounded-box bg-neutral p-3 text-sm text-neutral-content shadow-lg">
        {active?.download && (
          <>
            <div className="flex items-center justify-between gap-2 font-semibold">
              <span className="truncate">{t("downloads.downloading", { name: active.info.name })}</span>
              <button type="button" className="btn btn-ghost btn-xs" aria-label={t("models.cancel")} onClick={() => cancel(active.info.id)}>✕</button>
            </div>
            <div className="text-xs text-fg-muted">
              {Math.round((active.download.received / Math.max(1, active.download.total)) * 100)}% · {formatSize(active.download.received)} / {formatSize(active.download.total)}
            </div>
            <progress className="progress progress-primary mt-1 h-1 w-full" value={active.download.received} max={Math.max(1, active.download.total)} />
          </>
        )}
        {queue.map((id) => (
          <div key={id} className="mt-1 flex items-center justify-between gap-2 text-xs text-fg-muted">
            <span className="truncate">{t("downloads.next", { name: name(id) })}</span>
            <button type="button" className="btn btn-ghost btn-xs" aria-label={t("models.cancel")} onClick={() => dequeue(id)}>✕</button>
          </div>
        ))}
      </div>
    </div>
  );
}
