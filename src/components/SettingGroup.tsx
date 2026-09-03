import type { ReactNode } from "react";

export function SettingGroup({ children }: { children: ReactNode }) {
  return <div className="divide-y divide-base-300 rounded-box bg-base-200">{children}</div>;
}

export function SettingRow({ label, as = "label", children }: { label: string; as?: "label" | "div"; children: ReactNode }) {
  const Tag = as;
  return (
    <Tag className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-4 py-3 text-sm">
      <span>{label}</span>
      <span className="flex items-center gap-2 text-fg-muted">{children}</span>
    </Tag>
  );
}
