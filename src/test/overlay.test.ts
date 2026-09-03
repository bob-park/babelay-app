import { describe, it, expect } from "vitest";
import { overlayLines } from "../lib/overlay";

describe("overlayLines", () => {
  it("both shows source and partial", () => expect(overlayLines("both", "S", "P")).toEqual({ primary: "S", secondary: "P" }));
  it("source shows source and partial", () => expect(overlayLines("source", "S", "P")).toEqual({ primary: "S", secondary: "P" }));
  it("target is empty until a translation exists", () => expect(overlayLines("target", "S", "P")).toEqual({ primary: "", secondary: "" }));
  it("target shows the translation when given", () => expect(overlayLines("target", "S", "P", "T")).toEqual({ primary: "T", secondary: "" }));
});
