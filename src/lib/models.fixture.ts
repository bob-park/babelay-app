// ponytail: 2단계에서 엔진 레지스트리(get_models 커맨드)로 교체한다.
export interface ModelInfo {
  id: string;
  kind: "asr" | "llm";
  name: string;
  desc_key: string;
  size_bytes: number;
  speed: 1 | 2 | 3 | 4 | 5;
}

const MB = 1024 * 1024;
const GB = 1024 * MB;

export const ASR_MODELS: ModelInfo[] = [
  { id: "tiny", kind: "asr", name: "Whisper Tiny", desc_key: "models.desc.tiny", size_bytes: 75 * MB, speed: 5 },
  { id: "base", kind: "asr", name: "Whisper Base", desc_key: "models.desc.base", size_bytes: 142 * MB, speed: 4 },
  { id: "small", kind: "asr", name: "Whisper Small", desc_key: "models.desc.small", size_bytes: 466 * MB, speed: 3 },
  { id: "medium", kind: "asr", name: "Whisper Medium", desc_key: "models.desc.medium", size_bytes: 1.5 * GB, speed: 2 },
  { id: "large-v3-turbo", kind: "asr", name: "Whisper Large v3 Turbo", desc_key: "models.desc.large_v3_turbo", size_bytes: 1.6 * GB, speed: 2 },
  { id: "large-v3", kind: "asr", name: "Whisper Large v3", desc_key: "models.desc.large_v3", size_bytes: 3.1 * GB, speed: 1 },
];

export const LLM_MODELS: ModelInfo[] = [
  { id: "gemma3-1b", kind: "llm", name: "Gemma 3 1B", desc_key: "models.desc.gemma3_1b", size_bytes: 0.8 * GB, speed: 5 },
  { id: "qwen3.5-2b", kind: "llm", name: "Qwen 3.5 2B", desc_key: "models.desc.qwen3_5_2b", size_bytes: 1.4 * GB, speed: 4 },
  { id: "gemma3-4b", kind: "llm", name: "Gemma 3 4B", desc_key: "models.desc.gemma3_4b", size_bytes: 2.5 * GB, speed: 3 },
  { id: "qwen3.5-4b", kind: "llm", name: "Qwen 3.5 4B", desc_key: "models.desc.qwen3_5_4b", size_bytes: 2.5 * GB, speed: 3 },
];

export const BALANCED = { asr: "small", llm: "qwen3.5-2b" }; // 2단계에서 시스템 사양 기반으로 교체

export function formatSize(bytes: number): string {
  return bytes >= GB ? `${(bytes / GB).toFixed(1)} GB` : `${Math.round(bytes / MB)} MB`;
}
