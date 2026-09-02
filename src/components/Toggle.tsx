export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label?: string }) {
  return (
    <label className="flex cursor-pointer items-center justify-between gap-4">
      {label && <span>{label}</span>}
      <input
        type="checkbox"
        role="switch"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="peer sr-only"
      />
      <span className="relative h-6 w-11 rounded-full bg-surface-2 transition peer-checked:bg-accent peer-checked:[&>span]:left-[22px] peer-focus-visible:ring-2 peer-focus-visible:ring-accent">
        <span className="absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition" />
      </span>
    </label>
  );
}
