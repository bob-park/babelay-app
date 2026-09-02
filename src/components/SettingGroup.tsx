import type { ReactNode } from "react";

export function SettingGroup({ children }: { children: ReactNode }) {
  return <div className="divide-y divide-fg-muted/15 rounded-[var(--radius-card)] bg-base-2">{children}</div>;
}

export function SettingRow({ label, as = "label", children }: { label: string; as?: "label" | "div"; children: ReactNode }) {
  const Tag = as;
  return (
    <Tag className="flex items-center justify-between gap-4 px-4 py-3 text-sm">
      <span>{label}</span>
      <span className="flex items-center gap-2 text-fg-muted">{children}</span>
    </Tag>
  );
}
