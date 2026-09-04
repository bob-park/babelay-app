import { describe, it, expect } from "vitest";
import { langName, resolveLang } from "../lib/i18n";

describe("resolveLang", () => {
  it("uses navigator language for system", () => {
    expect(resolveLang("system", "ko-KR")).toBe("ko");
    expect(resolveLang("system", "ja")).toBe("ja");
    expect(resolveLang("system", "de-DE")).toBe("en");
  });
  it("explicit pref wins", () => {
    expect(resolveLang("ja", "ko-KR")).toBe("ja");
  });
});

describe("langName", () => {
  const t = (k: string) => `T:${k}`;
  it("uses the locale keys for the three supported languages", () => {
    expect(langName("ko", t, "en")).toBe("T:general.langKo");
    expect(langName("ja", t, "ko")).toBe("T:general.langJa");
  });
  it("falls back to Intl.DisplayNames in the UI language, then to the upper-cased code", () => {
    expect(langName("de", t, "ko")).toBe("독일어");
    expect(langName("zz-not-a-lang", t, "ko")).toBe("ZZ-NOT-A-LANG");
  });
});
