interface Props<T extends string> {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
}

// daisyUI join. 얇은 래퍼를 남기는 이유는 네 곳에서 같은 마크업을 반복하지 않기 위해서다.
export function SegmentedControl<T extends string>({ value, options, onChange }: Props<T>) {
  return (
    <div role="tablist" className="join">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="tab"
          aria-selected={o.value === value}
          onClick={() => onChange(o.value)}
          className={`btn btn-sm join-item ${o.value === value ? "btn-primary" : "btn-ghost"}`}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
