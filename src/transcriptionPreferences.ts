export const TRANSCRIPTION_PREFERENCES_KEY = "transcribe-options";

const MODELS = ["tiny", "base", "small", "medium", "large", "turbo"] as const;
const DEVICES = ["auto", "gpu", "cpu"] as const;

export type TranscriptionModel = (typeof MODELS)[number];
export type TranscriptionDevice = (typeof DEVICES)[number];

export interface TranscriptionPreferences {
  version: 1;
  model: TranscriptionModel;
  device: TranscriptionDevice;
  threads: number;
}

export type TranscriptionOptions = Omit<TranscriptionPreferences, "version">;

export const DEFAULT_TRANSCRIPTION_PREFERENCES: TranscriptionPreferences = {
  version: 1,
  model: "base",
  device: "auto",
  threads: 4,
};

function isModel(value: unknown): value is TranscriptionModel {
  return typeof value === "string" && (MODELS as readonly string[]).includes(value);
}

function isDevice(value: unknown): value is TranscriptionDevice {
  return typeof value === "string" && (DEVICES as readonly string[]).includes(value);
}

function isThreadCount(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= 512;
}

export function loadTranscriptionPreferences(
  storage: Pick<Storage, "getItem">,
): TranscriptionPreferences {
  try {
    const value: unknown = JSON.parse(
      storage.getItem(TRANSCRIPTION_PREFERENCES_KEY) ?? "null",
    );

    if (typeof value !== "object" || value === null) {
      return DEFAULT_TRANSCRIPTION_PREFERENCES;
    }

    const preferences = value as Record<string, unknown>;
    if (
      preferences.version !== 1 ||
      !isModel(preferences.model) ||
      !isDevice(preferences.device) ||
      !isThreadCount(preferences.threads)
    ) {
      return DEFAULT_TRANSCRIPTION_PREFERENCES;
    }

    return {
      version: 1,
      model: preferences.model,
      device: preferences.device,
      threads: preferences.threads,
    };
  } catch {
    return DEFAULT_TRANSCRIPTION_PREFERENCES;
  }
}

export function saveTranscriptionPreferences(
  storage: Pick<Storage, "setItem">,
  options: TranscriptionOptions,
) {
  storage.setItem(
    TRANSCRIPTION_PREFERENCES_KEY,
    JSON.stringify({ version: 1, ...options }),
  );
}
