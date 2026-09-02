import { invoke } from "@tauri-apps/api/core";
import type { DeepPartial, ModelStatus, Settings } from "./types";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  patchSettings: (patch: DeepPartial<Settings>) => invoke<void>("patch_settings", { patch }),
  getPlatform: () => invoke<string>("get_platform"),
  checkAudioPermission: () => invoke<"granted" | "denied" | "unknown">("check_audio_permission"),
  openPrivacySettings: () => invoke<void>("open_privacy_settings"),
  finishOnboarding: () => invoke<void>("finish_onboarding"),
  overlaySetAdjustMode: (enabled: boolean) => invoke<void>("overlay_set_adjust_mode", { enabled }),
  overlayCommitPosition: () => invoke<void>("overlay_commit_position"),
  getModels: () => invoke<ModelStatus[]>("get_models"),
  downloadModel: (id: string) => invoke<void>("download_model", { id }),
  cancelDownload: (id: string) => invoke<void>("cancel_download", { id }),
  deleteModel: (id: string) => invoke<void>("delete_model", { id }),
};
