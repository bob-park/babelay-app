import { describe, it, expect } from "vitest";
import { resolveTheme } from "../lib/theme";

describe("resolveTheme", () => {
  it("follows system when pref is system", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });
  it("explicit pref wins", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });
});
