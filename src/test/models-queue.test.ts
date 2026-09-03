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
  info: { id, kind, name: id, desc_key: "models.desc.small", size_bytes: 10, speed: 3, url: "https://x", filename: id, sha256: null },
  installed: false, in_use: false, balanced: false, download,
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

  it("queues while another download is active, replacing the same kind", async () => {
    useModels.setState({ models: [model("small", "asr", { received: 1, total: 10 }), model("qwen", "llm"), model("gemma", "llm")] });
    await useModels.getState().enqueue("qwen");
    await useModels.getState().enqueue("gemma");
    expect(h.api.downloadModel).not.toHaveBeenCalled();
    expect(useModels.getState().queue).toEqual(["gemma"]);
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
});
