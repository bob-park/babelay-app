const paths = {
  live: "M4 6h16v9H4z M8 19h8 M12 15v4",
  history: "M4 6h16 M4 12h10 M4 18h13",
  general: "M12 3v2 M12 19v2 M3 12h2 M19 12h2 M6 6l1.5 1.5 M16.5 16.5 18 18 M6 18l1.5-1.5 M16.5 7.5 18 6 M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z",
  models: "M4 7h16v4H4z M4 13h16v4H4z M7 9h.01 M7 15h.01",
  translation: "M4 5h8 M8 5v2c0 4-2 7-5 9 M6 10c1 3 3 5 6 6 M13 19l4-9 4 9 M14.5 16h5",
  overlay: "M3 5h18v11H3z M7 20h10 M8 13h8",
  collapse: "M14 6l-6 6 6 6",
  search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14z M20 20l-4-4",
  info: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z M12 11v5 M12 8h.01",
  help: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.7.4-1 1-1 1.7 M12 17h.01",
  trash: "M4 7h16 M10 11v6 M14 11v6 M6 7l1 13h10l1-13 M9 7V4h6v3",
  check: "M5 12l4 4L19 7",
  x: "M6 6l12 12 M18 6L6 18",
  play: "M8 5.5v13l10-6.5z",
} as const;

export type IconName = keyof typeof paths;

export function Icon({ name, className = "h-4 w-4" }: { name: IconName; className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d={paths[name]} />
    </svg>
  );
}
