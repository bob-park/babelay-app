import type { Theme } from "./types";

export function resolveTheme(pref: Theme, systemDark: boolean): "dark" | "light" {
  if (pref === "system") return systemDark ? "dark" : "light";
  return pref;
}

export function applyTheme(pref: Theme) {
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const apply = () => document.documentElement.classList.toggle("dark", resolveTheme(pref, mq.matches) === "dark");
  apply();
  mq.onchange = apply;
}
