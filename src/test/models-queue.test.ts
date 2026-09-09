import { describe, it, expect, vi, beforeEach } from "vitest";
import type { DownloadEvent, ModelStatus } from "../lib/types";

const h = vi.hoisted(() => ({
  handler: null as ((e: { payload: DownloadEvent }) => void) | null,
  api: { getModels: vi.fn(), downloadModel: vi.fn(), cancelDownload: vi.fn(), deleteModel: vi.fn() },
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((_: string, cb: (e: { payload: DownloadEvent }) => void) => { h.handler = cb; return Promise.resolve(() => {}); }),
}));
vi.mock("../lib/tauri", () => ({ api: h.api }));

import { useModels } from "../lib/models";

const model = (id: string, kind: "asr" | "llm", download: ModelStatus["download"] = null): ModelStatus => ({
  info: { id, kind, name: id, desc_key: "models.desc.small", size_bytes: 10, total_bytes: 10, speed: 3, quality: 3, url: "https://x", filename: id, sha256: null, mmproj: null },
  installed: false, in_use: false, balanced: false, fit: "good", download,
});
const flush = () => new Promise((r) => setTimeout(r, 0));

beforeEach(() => {
  vi.clearAllMocks();
  h.api.getModels.mockResolvedValue([model("small", "asr"), model("qwen", "llm"), model("gemma", "llm")]);
  h.api.downloadModel.mockResolvedValue(undefined);
  useModels.setState({ models: [model("small", "asr"), model("qwen", "llm"), model("gemma", "llm")], queue: [], lastEvent: null });
});

describe("download queue", () => {
  it("starts immediately when nothing is downloading", async () => {
    await useModels.getState().enqueue("small");
    expect(h.api.downloadModel).toHaveBeenCalledWith("small");
    expect(useModels.getState().queue).toEqual([]);
  });

  it("queues in order while another download is active", async () => {
    useModels.setState({ models: [model("small", "asr", { received: 1, total: 10 }), model("qwen", "llm"), model("gemma", "llm")] });
    await useModels.getState().enqueue("qwen");
    await useModels.getState().enqueue("gemma");
    await useModels.getState().enqueue("qwen");
    expect(h.api.downloadModel).not.toHaveBeenCalled();
    expect(useModels.getState().queue).toEqual(["qwen", "gemma"]);
  });

  it("replaceKind swaps waiting models of the same kind", async () => {
    // 다른 종류(asr)의 대기 항목은 남고, 같은 종류(llm)만 교체된다.
    useModels.setState({
      models: [model("small", "asr", { received: 1, total: 10 }), model("small2", "asr"), model("qwen", "llm"), model("gemma", "llm")],
      queue: ["small2"],
    });
    await useModels.getState().enqueue("qwen", { replaceKind: true });
    await useModels.getState().enqueue("gemma", { replaceKind: true });
    expect(useModels.getState().queue).toEqual(["small2", "gemma"]);
  });

  it("remove drops a waiting model from the queue first", async () => {
    h.api.deleteModel.mockResolvedValue(undefined);
    useModels.setState({ queue: ["qwen", "gemma"] });
    await useModels.getState().remove("qwen");
    expect(useModels.getState().queue).toEqual(["gemma"]);
    expect(h.api.deleteModel).toHaveBeenCalledWith("qwen");
  });

  it("starts the next queued model after the active one finishes", async () => {
    useModels.setState({ models: [model("small", "asr", { received: 1, total: 10 }), model("qwen", "llm"), model("gemma", "llm")], queue: ["qwen"] });
    const unbind = useModels.getState().bind();
    await flush();
    h.handler!({ payload: { id: "small", received: 10, total: 10, state: "done", message: null } });
    await flush();
    expect(h.api.downloadModel).toHaveBeenCalledWith("qwen");
    expect(useModels.getState().queue).toEqual([]);
    unbind();
  });

  it("dequeue removes a waiting model", () => {
    useModels.setState({ queue: ["qwen", "gemma"] });
    useModels.getState().dequeue("qwen");
    expect(useModels.getState().queue).toEqual(["gemma"]);
  });

  it("two un-awaited enqueues start one download and queue the other", async () => {
    let resolveDl: () => void = () => {};
    h.api.downloadModel.mockImplementationOnce(() => new Promise<void>((r) => { resolveDl = r; }));
    const p1 = useModels.getState().enqueue("small");
    const p2 = useModels.getState().enqueue("qwen");
    expect(h.api.downloadModel).toHaveBeenCalledTimes(1);
    expect(useModels.getState().queue).toEqual(["qwen"]);
    resolveDl();
    await Promise.all([p1, p2]);
  });

  it("enqueue of the active model is a no-op", async () => {
    useModels.setState({ models: [model("small", "asr", { received: 1, total: 10 }), model("qwen", "llm"), model("gemma", "llm")] });
    await useModels.getState().enqueue("small");
    expect(h.api.downloadModel).not.toHaveBeenCalled();
    expect(useModels.getState().queue).toEqual([]);
  });
});
