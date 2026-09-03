import { useState } from "react";
import { useTranslation } from "react-i18next";
import { formatSize, rowAction, useModels } from "../lib/models";
import type { ModelStatus } from "../lib/types";
import { ConfirmModal } from "./ConfirmModal";
import { Icon } from "./icons";

interface Props {
  status: ModelStatus;
  selected: boolean;
  onSelect: () => void;
}

export function ModelRow({ status, selected, onSelect }: Props) {
  const { t } = useTranslation();
  const { download, cancel, remove } = useModels();
  // 슬롯은 하나뿐이라 다른 모델을 받는 중이면 백엔드가 "busy" 로 거절한다.
  const busy = useModels((st) => st.models.some((m) => m.download));
  const action = rowAction(status);
  const [confirm, setConfirm] = useState(false);
  const { info } = status;
  const pct = status.download ? Math.round((status.download.received / Math.max(1, status.download.total)) * 100) : null;

  const meta = status.download
    ? `${formatSize(info.size_bytes)} · ${t("models.downloading")} · ${pct}% · ${formatSize(status.download.received)} / ${formatSize(status.download.total)}`
    : `${formatSize(info.size_bytes)} · ${t(info.desc_key)}`;

  const deleteBtn = (
    <button type="button" className="btn btn-ghost btn-sm gap-1" aria-label={t("models.delete")} onClick={() => setConfirm(true)}>
      <Icon name="trash" />{t("models.delete")}
    </button>
  );
  const button = {
    download: <button type="button" className="btn btn-primary btn-sm" disabled={busy} onClick={() => download(info.id)}>{t("models.download")}</button>,
    cancel: <button type="button" className="btn btn-ghost btn-sm" onClick={() => cancel(info.id)}>{t("models.cancel")}</button>,
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
      className={`grid grid-cols-[1fr_auto] items-center gap-3 rounded-box bg-base-200 px-4 py-3 text-left text-sm hover:bg-base-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${selected ? "ring-[1.5px] ring-inset ring-primary" : ""}`}
    >
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2 font-semibold">
          <span className="truncate">{info.name}</span>
          {status.in_use && status.installed && <span className="badge badge-primary badge-sm">{t("models.badgeInUse")}</span>}
          {status.installed && !status.in_use && <span className="badge badge-neutral badge-sm">{t("models.badgeInstalled")}</span>}
          {status.balanced && <span className="badge badge-neutral badge-sm">{t("models.badgeRecommended")}</span>}
        </div>
        <div className="mt-0.5 flex items-center gap-2 text-xs text-fg-muted">
          <span className="truncate">{meta}</span>
          {!status.download && (
            <span className="inline-flex gap-0.5" aria-label={t(`models.speed${info.speed}`)}>
              {[1, 2, 3, 4, 5].map((i) => <span key={i} className={`h-1.5 w-1.5 rounded-[2px] ${i <= info.speed ? "bg-fg-muted" : "bg-neutral"}`} />)}
            </span>
          )}
        </div>
        {status.download && <progress className="progress progress-primary mt-2 h-1 w-full" value={status.download.received} max={Math.max(1, status.download.total)} />}
      </div>
      <div onClick={(e) => e.stopPropagation()}>
        {button}
        <ConfirmModal
          open={confirm}
          message={t("models.confirmDelete", { name: info.name, size: formatSize(info.size_bytes) })}
          onCancel={() => setConfirm(false)}
          onConfirm={() => { setConfirm(false); remove(info.id); }}
        />
      </div>
    </div>
  );
}
