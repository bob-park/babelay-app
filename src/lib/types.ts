export type Theme = "system" | "dark" | "light";
export type UiLang = "system" | "ko" | "en" | "ja";
export type Lang = "ko" | "en" | "ja";
export type SourceLang = "auto" | Lang;
export type DisplayMode = "both" | "source" | "target";
export type Provider = "openai" | "anthropic" | "gemini" | "deepl" | "custom";

export interface Settings {
  version: number;
  general: { theme: Theme; ui_language: UiLang; onboarding_done: boolean };
  asr: { model_id: string; gpu: boolean; source_lang: SourceLang };
  translation: {
    backend: "local" | "cloud";
    local_model: string;
    cloud: { provider: Provider; model: string; base_url: string };
  };
  overlay: {
    enabled: boolean;
    monitor_id: string;
    x_ratio: number;
    y_ratio: number;
    w_ratio: number;
    display_mode: DisplayMode;
    subtitle_lang: UiLang;
    font_size: number;
    bg_opacity: number;
  };
}

export type DeepPartial<T> = { [K in keyof T]?: T[K] extends object ? DeepPartial<T[K]> : T[K] };

export type ModelKind = "asr" | "llm";
export interface ModelInfo { id: string; kind: ModelKind; name: string; desc_key: string; size_bytes: number; speed: 1 | 2 | 3 | 4 | 5; url: string; filename: string; sha256: string | null }
export interface DownloadProgress { received: number; total: number }
export interface ModelStatus { info: ModelInfo; installed: boolean; in_use: boolean; balanced: boolean; download: DownloadProgress | null }
export type DownloadState = "downloading" | "done" | "error" | "cancelled";
export interface DownloadEvent { id: string; received: number; total: number; state: DownloadState; message: string | null }

export type EngineEvent =
  | { type: "started"; gpu_active: boolean; gpu_fallback: boolean; model_id: string; source_lang: string | null; target_lang: string | null }
  | { type: "partial"; text: string; lang: string; start_ms: number }
  | { type: "final"; id: number; text: string; lang: string; start_ms: number; end_ms: number }
  | { type: "translated"; id: number; text: string; lang: string }
  | { type: "cpu_fallback"; stage: string }
  | { type: "lagging"; queued_ms: number }
  | { type: "error"; code: string; message: string }
  | { type: "stopped" };

export interface SessionSummary { id: number; started_at: number; ended_at: number | null; src_lang: string; tgt_lang: string; asr_model: string; translator: string | null; segments: number }
export interface SegmentRow { id: number; session_id: number; t0_ms: number; t1_ms: number; lang: string; src_text: string; tgt_text: string | null }
/** 연결 테스트 결과. 실패면 error 에 코드(ERROR_KEYS), text 에 상세. */
export interface TestTranslationResult { ok: boolean; ms: number; text: string; error: string | null }
export interface HwInfo { chip: string; mem_gb: number; gpu: string | null; gpu_mem_gb: number | null }
