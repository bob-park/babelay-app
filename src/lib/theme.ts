import type { Theme } from "./types";

export function resolveTheme(pref: Theme, systemDark: boolean): "dark" | "light" {
  if (pref === "system") return systemDark ? "dark" : "light";
  return pref;
}

// 모듈 레벨로 하나만 유지한다. matchMedia는 호출마다 새 객체를 돌려주므로
// applyTheme마다 새로 만들면 옛 pref를 붙든 핸들러가 쌓인다.
let mq: MediaQueryList | null = null;
let currentPref: Theme = "system";

function apply() {
  const dark = resolveTheme(currentPref, mq?.matches ?? false) === "dark";
  document.documentElement.classList.toggle("dark", dark);
}

export function applyTheme(pref: Theme) {
  currentPref = pref;
  if (!mq) {
    mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.onchange = apply;
  }
  apply();
}
