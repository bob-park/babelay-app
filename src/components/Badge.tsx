export function Badge({ tone = "muted", children }: { tone?: "accent" | "muted"; children: React.ReactNode }) {
  const cls = tone === "accent" ? "bg-accent text-accent-fg" : "bg-surface-2 text-fg";
  return <span className={`rounded-[2px] px-1.5 py-px text-[10.5px] font-semibold capitalize ${cls}`}>{children}</span>;
}
