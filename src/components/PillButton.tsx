import type { ButtonHTMLAttributes } from "react";

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "default" | "outline" | "ghost";
  size?: "sm" | "md";
};

const variants = {
  primary: "bg-accent text-accent-fg hover:brightness-110",
  default: "bg-surface text-fg hover:bg-surface-2",
  outline: "bg-transparent text-fg border border-fg-muted/60 hover:bg-surface",
  ghost: "bg-transparent text-fg-muted hover:text-fg",
};

export function PillButton({ variant = "default", size = "md", className = "", ...rest }: Props) {
  const pad = size === "sm" ? "px-3 py-1 text-[11px]" : "px-4 py-2 text-xs";
  return (
    <button
      {...rest}
      className={`rounded-full font-semibold transition disabled:opacity-40 ${pad} ${variants[variant]} ${className}`}
    />
  );
}
