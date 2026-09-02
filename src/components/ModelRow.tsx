import { useTranslation } from "react-i18next";
import { Badge } from "./Badge";
import { PillButton } from "./PillButton";
import { ProgressBar } from "./ProgressBar";
import { formatSize, rowAction, useModels } from "../lib/models";
import type { ModelStatus } from "../lib/types";

interface Props {
  status: ModelStatus;
  selected: boolean;
  onSelect: () => void;
}

export function ModelRow({ status, selected, onSelect }: Props) {
  const { t } = useTranslation();
  const { download, cancel, remove } = useModels();
  const action = rowAction(status);
  const { info } = status;
  const pct = status.download ? Math.round((status.download.received / Math.max(1, status.download.total)) * 100) : null;

  const meta = status.download
    ? `${formatSize(info.size_bytes)} · ${t("models.downloading")} · ${pct}% · ${formatSize(status.download.received)} / ${formatSize(status.download.total)}`
    : `${formatSize(info.size_bytes)} · ${t(info.desc_key)}`;

  const button = {
    download: <PillButton size="sm" variant="primary" onClick={() => download(info.id)}>{t("models.download")}</PillButton>,
    cancel: <PillButton size="sm" variant="ghost" onClick={() => cancel(info.id)}>{t("models.cancel")}</PillButton>,
    select: <PillButton size="sm" onClick={onSelect}>{t("models.select")}</PillButton>,
    delete: <PillButton size="sm" variant="ghost" disabled={status.in_use} onClick={() => remove(info.id)}>{t("models.delete")}</PillButton>,
  }[action];

  return (
    <div
      role="button"
      tabIndex={0}
      aria-pressed={selected}
      onClick={onSelect}
      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect(); } }}
      className={`grid grid-cols-[1fr_auto] items-center gap-3 rounded-[var(--radius-card)] bg-base-2 px-4 py-3 text-left text-sm hover:bg-surface ${selected ? "ring-[1.5px] ring-inset ring-accent" : ""}`}
    >
      <div className="min-w-0">
        <div className="flex items-center gap-2 font-semibold">
          <span className="truncate">{info.name}</span>
          {status.in_use && <Badge tone="accent">{t("models.badgeInUse")}</Badge>}
          {status.installed && !status.in_use && <Badge>{t("models.badgeInstalled")}</Badge>}
          {status.balanced && <Badge>{t("models.badgeRecommended")}</Badge>}
        </div>
        <div className="mt-0.5 flex items-center gap-2 text-xs text-fg-muted">
          <span className="truncate">{meta}</span>
          {!status.download && (
            <span className="inline-flex gap-0.5" aria-label={t(`models.speed${info.speed}`)}>
              {[1, 2, 3, 4, 5].map((i) => <span key={i} className={`h-1.5 w-1.5 rounded-[2px] ${i <= info.speed ? "bg-fg-muted" : "bg-surface-2"}`} />)}
            </span>
          )}
        </div>
        {status.download && <div className="mt-2"><ProgressBar value={status.download.received} max={status.download.total} /></div>}
      </div>
      <div onClick={(e) => e.stopPropagation()}>{button}</div>
    </div>
  );
}
