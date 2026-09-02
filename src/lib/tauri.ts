import { invoke } from "@tauri-apps/api/core";
import type { DeepPartial, MonitorInfo, Settings } from "./types";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  patchSettings: (patch: DeepPartial<Settings>) => invoke<void>("patch_settings", { patch }),
  getPlatform: () => invoke<string>("get_platform"),
  checkAudioPermission: () => invoke<"granted" | "denied" | "unknown">("check_audio_permission"),
  openPrivacySettings: () => invoke<void>("open_privacy_settings"),
  finishOnboarding: () => invoke<void>("finish_onboarding"),
  overlaySetAdjustMode: (enabled: boolean) => invoke<void>("overlay_set_adjust_mode", { enabled }),
  overlayGetMonitors: () => invoke<MonitorInfo[]>("overlay_get_monitors"),
  overlayCommitPosition: () => invoke<void>("overlay_commit_position"),
};
