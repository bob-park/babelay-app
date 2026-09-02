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

export interface MonitorInfo {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  primary: boolean;
}

export type ModelKind = "asr" | "llm";
export interface ModelInfo { id: string; kind: ModelKind; name: string; desc_key: string; size_bytes: number; speed: 1 | 2 | 3 | 4 | 5; url: string; filename: string; sha256: string | null }
export interface DownloadProgress { received: number; total: number }
export interface ModelStatus { info: ModelInfo; installed: boolean; in_use: boolean; balanced: boolean; download: DownloadProgress | null }
export type DownloadState = "downloading" | "done" | "error" | "cancelled";
export interface DownloadEvent { id: string; received: number; total: number; state: DownloadState; message: string | null }
