import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, it, expect } from "vitest";
import ko from "../locales/ko.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

function keys(obj: Record<string, unknown>, prefix = ""): string[] {
  return Object.entries(obj).flatMap(([k, v]) =>
    typeof v === "object" && v !== null ? keys(v as Record<string, unknown>, `${prefix}${k}.`) : [`${prefix}${k}`],
  );
}

// 코드에서 키를 조립해 쓰는 곳. 리터럴로는 절대 안 잡힌다.
const DYNAMIC = ["models.desc.", "models.preset.", "onboarding.step.", "onboarding.title.", "settings.", "models.speed", "translation.provider"];

function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = join(dir, e.name);
    if (e.isDirectory()) return p.endsWith("/locales") || p.endsWith("/test") ? [] : sources(p);
    return /\.tsx?$/.test(e.name) ? [readFileSync(p, "utf8")] : [];
  });
}

describe("locale files", () => {
  it("have identical key sets", () => {
    const e = keys(en).sort();
    expect(keys(ko).sort()).toEqual(e);
    expect(keys(ja).sort()).toEqual(e);
  });
  it("have no empty strings", () => {
    for (const f of [ko, en, ja]) {
      const flat = JSON.stringify(f);
      expect(flat).not.toContain('""');
    }
  });
  it("every locale key is used in src", () => {
    const blob = sources("src").join("\n");
    const orphans = keys(en).filter((k) => !DYNAMIC.some((p) => k.startsWith(p)) && !blob.includes(k));
    expect(orphans).toEqual([]);
  });
});
