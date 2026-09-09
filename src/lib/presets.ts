import type { ModelStatus, PresetId, PresetStatus } from "./types";

/** 현재 설정(전사·번역)과 두 모델이 모두 같은 프리셋. 없으면 null. */
export function matchPreset(presets: PresetStatus[], asrId: string, llmId: string): PresetId | null {
  return presets.find((p) => p.asr === asrId && p.llm === llmId)?.id ?? null;
}

/** 프리셋의 두 모델이 모두 설치됐는지 / 모두 사용 중인지. */
export function presetState(p: PresetStatus, models: ModelStatus[]): { installed: boolean; inUse: boolean } {
  const both = [p.asr, p.llm].map((id) => models.find((m) => m.info.id === id));
  return {
    installed: both.every((m) => m?.installed),
    inUse: both.every((m) => m?.installed && m.in_use),
  };
}
