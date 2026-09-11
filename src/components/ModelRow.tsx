import { useState } from "react";
import { useTranslation } from "react-i18next";
import { formatSize, rowAction, useModels } from "../lib/models";
import type { ModelStatus } from "../lib/types";
import { ConfirmModal } from "./ConfirmModal";
import { Icon } from "./icons";

/** 1~5 등급을 점 5개로. 품질은 초록, 속도는 회색. */
function Meter({ value, accent, label }: { value: number; accent?: boolean; label: string }) {
  return (
    <span className="inline-flex items-center gap-1" aria-label={label}>
      <span className="inline-flex gap-0.5">
        {[1, 2, 3, 4, 5].map((i) => <span key={i} className={`h-1.5 w-1.5 rounded-[2px] ${i <= value ? (accent ? "bg-primary" : "bg-fg-muted") : "bg-base-300"}`} />)}
      </span>
    </span>
  );
}

interface Props {
  status: ModelStatus;
  selected: boolean;
  onSelect: () => void;
}

export function ModelRow({ status, selected, onSelect }: Props) {
  const { t } = useTranslation();
  const { enqueue, dequeue, cancel, remove } = useModels();
  const queued = useModels((st) => st.queue.includes(status.info.id));
  const action = rowAction(status, queued);
  const [confirm, setConfirm] = useState(false);
  const { info } = status;
  const pct = status.download ? Math.round((status.download.received / Math.max(1, status.download.total)) * 100) : null;

  const size = formatSize(info.total_bytes);
  const meta = status.download
    ? `${size} · ${t("models.downloading")} · ${pct}% · ${formatSize(status.download.received)} / ${formatSize(status.download.total)}`
    : size;

  const deleteBtn = (
    <button type="button" className="btn btn-ghost btn-sm gap-1" aria-label={t("models.delete")} onClick={() => setConfirm(true)}>
      <Icon name="trash" />{t("models.delete")}
    </button>
  );
  const button = {
    download: <button type="button" className="btn btn-primary btn-sm" onClick={() => enqueue(info.id)}>{t("models.download")}</button>,
    cancel: <button type="button" className="btn btn-ghost btn-sm" onClick={() => cancel(info.id)}>{t("models.cancel")}</button>,
    queued: <button type="button" className="btn btn-ghost btn-sm gap-1" onClick={() => dequeue(info.id)}>{t("models.queued")}<Icon name="x" className="h-3 w-3" /></button>,
    select: (
      <div className="flex flex-wrap gap-1">
        <button type="button" className="btn btn-primary btn-sm" onClick={onSelect}>{t("models.select")}</button>
        {deleteBtn}
      </div>
    ),
    delete: deleteBtn,
  }[action];

  return (
    <div
      role="button"
      tabIndex={0}
      aria-pressed={selected}
      onClick={onSelect}
      onKeyDown={(e) => { if (e.target !== e.currentTarget) return; if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect(); } }}
      className={`grid grid-cols-[1fr_auto] items-center gap-3 rounded-box border border-base-300 bg-base-100 px-4 py-3 text-left text-sm shadow-kr hover:bg-base-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${selected ? "border-primary ring-1 ring-inset ring-primary" : ""}`}
    >
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2 font-semibold">
          <span className="truncate">{info.name}</span>
          {status.in_use && status.installed && <span className="badge badge-success badge-soft badge-sm">{t("models.badgeInUse")}</span>}
          {status.installed && !status.in_use && <span className="badge badge-ghost badge-sm">{t("models.badgeInstalled")}</span>}
          {status.balanced && <span className="badge badge-secondary badge-sm">{t("models.badgeRecommended")}</span>}
          {status.fit === "heavy" && <span className="badge badge-warning badge-soft badge-sm">{t("models.badgeHeavy")}</span>}
        </div>
        <div className="mt-0.5 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-fg-muted">
          <span className="truncate">{meta}</span>
          {!status.download && (
            <>
              <span className="inline-flex items-center gap-1">{t("models.quality")} <Meter value={info.quality} accent label={`${t("models.quality")} ${info.quality}/5`} /></span>
              <span className="inline-flex items-center gap-1">{t("models.speed")} <Meter value={info.speed} label={t(`models.speed${info.speed}`)} /></span>
            </>
          )}
        </div>
        <div className="text-xs text-fg-muted">{t(info.desc_key)}</div>
        {status.download && <progress className="progress progress-primary mt-2 h-1 w-full" value={status.download.received} max={Math.max(1, status.download.total)} />}
      </div>
      <div onClick={(e) => e.stopPropagation()}>
        {button}
        <ConfirmModal
          open={confirm}
          message={t("models.confirmDelete", { name: info.name, size: formatSize(info.total_bytes) })}
          onCancel={() => setConfirm(false)}
          onConfirm={() => { setConfirm(false); remove(info.id); }}
        />
      </div>
    </div>
  );
}
