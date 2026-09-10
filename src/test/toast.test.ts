import { describe, it, expect, vi, beforeEach } from "vitest";
import { toast } from "react-toastify";
import { showError } from "../lib/toast";

vi.mock("react-toastify", () => ({ toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn(), info: vi.fn() }) }));

beforeEach(() => vi.clearAllMocks());

describe("showError", () => {
  it("uses Error.message", () => {
    showError(new Error("disk full"));
    expect(toast.error).toHaveBeenCalledWith("disk full");
  });
  it("stringifies non-Error values", () => {
    showError("busy_stopping");
    expect(toast.error).toHaveBeenCalledWith("busy_stopping");
  });
});
