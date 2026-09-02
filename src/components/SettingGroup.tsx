import type { ReactNode } from "react";

export function SettingGroup({ children }: { children: ReactNode }) {
  return <div className="divide-y divide-surface rounded-[var(--radius-card)] bg-base-2">{children}</div>;
}

export function SettingRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex items-center justify-between gap-4 px-4 py-3 text-sm">
      <span>{label}</span>
      <span className="flex items-center gap-2 text-fg-muted">{children}</span>
    </label>
  );
}
