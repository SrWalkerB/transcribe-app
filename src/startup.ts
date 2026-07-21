export type Platform = "windows" | "macos" | "linux";

export interface DependencyStatus {
  ffmpeg: boolean;
  python: boolean;
  faster_whisper: boolean;
  whisper_cli: boolean;
  model: boolean;
}

export const SETUP_COMPLETE_KEY = "transcribe-setup-complete";

export function dependenciesReady(
  platform: Platform,
  dependencies: DependencyStatus | null,
) {
  if (platform === "windows") {
    return Boolean(
      dependencies?.ffmpeg && dependencies?.whisper_cli && dependencies?.model,
    );
  }

  return Boolean(
    dependencies?.ffmpeg &&
      dependencies?.python &&
      dependencies?.faster_whisper,
  );
}

export function decideStartupState(
  setupComplete: boolean,
  platform: Platform,
  dependencies: DependencyStatus | null,
) {
  return setupComplete && dependenciesReady(platform, dependencies)
    ? "idle"
    : "settings";
}
