// ponytail: 2단계에서 엔진 레지스트리(get_models 커맨드)로 교체한다.
export interface ModelInfo {
  id: string;
  kind: "asr" | "llm";
  name: string;
  desc: string;
  size_bytes: number;
  speed: 1 | 2 | 3 | 4 | 5;
}

const MB = 1024 * 1024;
const GB = 1024 * MB;

export const ASR_MODELS: ModelInfo[] = [
  { id: "tiny", kind: "asr", name: "Whisper Tiny", desc: "fastest, low accuracy", size_bytes: 75 * MB, speed: 5 },
  { id: "base", kind: "asr", name: "Whisper Base", desc: "fast, short sentences", size_bytes: 142 * MB, speed: 4 },
  { id: "small", kind: "asr", name: "Whisper Small", desc: "balanced speed and accuracy", size_bytes: 466 * MB, speed: 3 },
  { id: "medium", kind: "asr", name: "Whisper Medium", desc: "high accuracy", size_bytes: 1.5 * GB, speed: 2 },
  { id: "large-v3-turbo", kind: "asr", name: "Whisper Large v3 Turbo", desc: "high accuracy, strong multilingual", size_bytes: 1.6 * GB, speed: 2 },
  { id: "large-v3", kind: "asr", name: "Whisper Large v3", desc: "best accuracy", size_bytes: 3.1 * GB, speed: 1 },
];

export const LLM_MODELS: ModelInfo[] = [
  { id: "gemma3-1b", kind: "llm", name: "Gemma 3 1B", desc: "fastest, simple sentences", size_bytes: 0.8 * GB, speed: 5 },
  { id: "qwen3.5-2b", kind: "llm", name: "Qwen 3.5 2B", desc: "good balance", size_bytes: 1.4 * GB, speed: 4 },
  { id: "gemma3-4b", kind: "llm", name: "Gemma 3 4B", desc: "better fluency", size_bytes: 2.5 * GB, speed: 3 },
  { id: "qwen3.5-4b", kind: "llm", name: "Qwen 3.5 4B", desc: "best quality, strong CJK", size_bytes: 2.5 * GB, speed: 3 },
];

export const BALANCED = { asr: "small", llm: "qwen3.5-2b" }; // 2단계에서 시스템 사양 기반으로 교체

export function formatSize(bytes: number): string {
  return bytes >= GB ? `${(bytes / GB).toFixed(1)} GB` : `${Math.round(bytes / MB)} MB`;
}
