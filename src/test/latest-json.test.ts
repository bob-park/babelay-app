import { describe, it, expect } from "vitest";
import { buildManifest } from "../../scripts/latest-json.mjs";

const now = new Date("2026-09-10T00:00:00Z");
const mac = { name: "Babelay.app.tar.gz.sig", body: "MACSIG\n" };
const win = { name: "Babelay_0.3.0_x64-setup.exe.sig", body: "WINSIG" };

describe("buildManifest", () => {
  it("maps sig files to updater platform entries", () => {
    const m = buildManifest("v0.3.0", [mac, win], now);
    expect(m.version).toBe("0.3.0");
    expect(m.pub_date).toBe("2026-09-10T00:00:00.000Z");
    expect(m.platforms).toEqual({
      "darwin-aarch64": { signature: "MACSIG", url: "https://github.com/bob-park/babelay-app/releases/download/v0.3.0/Babelay.app.tar.gz" },
      "windows-x86_64": { signature: "WINSIG", url: "https://github.com/bob-park/babelay-app/releases/download/v0.3.0/Babelay_0.3.0_x64-setup.exe" },
    });
  });

  it("includes only platforms whose signature exists", () => {
    const m = buildManifest("v0.3.0", [mac], now);
    expect(Object.keys(m.platforms)).toEqual(["darwin-aarch64"]);
  });

  it("ignores unrelated files", () => {
    const m = buildManifest("v0.3.0", [{ name: "latest.json", body: "{}" }], now);
    expect(m.platforms).toEqual({});
  });
});
