// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { applyTheme } from "../lib/theme";

describe("applyTheme", () => {
  it("applies the pref and registers exactly one media-query listener", () => {
    let assigned = 0;
    let handler: (() => void) | null = null;
    const mql = { matches: true };
    Object.defineProperty(mql, "onchange", {
      set(fn: () => void) {
        assigned++;
        handler = fn;
      },
      get() {
        return handler;
      },
    });
    const matchMedia = vi.fn(() => mql);
    vi.stubGlobal("matchMedia", matchMedia);

    const theme = () => document.documentElement.dataset.theme;

    applyTheme("dark");
    expect(theme()).toBe("babelay");
    applyTheme("light");
    expect(theme()).toBe("babelay-light");
    applyTheme("system"); // matches: true
    expect(theme()).toBe("babelay");

    expect(matchMedia).toHaveBeenCalledTimes(1);
    expect(assigned).toBe(1);

    // 등록된 핸들러는 최신 pref를 읽는다.
    mql.matches = false;
    handler!();
    expect(theme()).toBe("babelay-light");
  });
});
