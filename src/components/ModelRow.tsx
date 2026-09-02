import { useTranslation } from "react-i18next";
import { Badge } from "./Badge";
import { formatSize, type ModelInfo } from "../lib/models.fixture";

interface Props {
  model: ModelInfo;
  selected: boolean;
  badges?: { balanced?: boolean; installed?: boolean; inUse?: boolean };
  onSelect: () => void;
}

export function ModelRow({ model, selected, badges = {}, onSelect }: Props) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      className={`grid w-full grid-cols-[16px_1.4fr_1.6fr_0.6fr_0.9fr] items-center gap-3 rounded-lg bg-base-2 px-3 py-2 text-left text-sm hover:bg-surface ${selected ? "ring-1 ring-accent bg-surface" : ""}`}
    >
      <span className={`h-3 w-3 rounded-full border ${selected ? "border-accent bg-accent" : "border-fg-muted"}`} />
      <span className="flex items-center gap-2 font-bold">
        {model.name}
        {badges.balanced && <Badge tone="accent">{t("models.badgeBalanced")}</Badge>}
        {badges.inUse && <Badge>{t("models.badgeInUse")}</Badge>}
        {badges.installed && !badges.inUse && <Badge>{t("models.badgeInstalled")}</Badge>}
      </span>
      <span className="text-fg-muted">{t(model.desc_key)}</span>
      <span className="text-fg-muted">{formatSize(model.size_bytes)}</span>
      <span className="flex items-center gap-2 text-fg-muted">
        <span className="relative h-1.5 w-9 rounded bg-surface-2">
          <span className="absolute inset-y-0 left-0 rounded bg-fg-muted" style={{ width: `${model.speed * 20}%` }} />
        </span>
        {t(`models.speed${model.speed}`)}
      </span>
    </button>
  );
}
