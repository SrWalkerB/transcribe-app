import { describe, expect, it } from "vitest";
import {
  DEFAULT_TRANSCRIPTION_PREFERENCES,
  loadTranscriptionPreferences,
} from "./transcriptionPreferences";

describe("loadTranscriptionPreferences", () => {
  it("restores a valid saved preference", () => {
    const storage = {
      getItem: () =>
        JSON.stringify({
          version: 1,
          model: "large",
          device: "gpu",
          threads: 12,
        }),
    };

    expect(loadTranscriptionPreferences(storage)).toEqual({
      version: 1,
      model: "large",
      device: "gpu",
      threads: 12,
    });
  });

  it("falls back to defaults for invalid saved data", () => {
    expect(loadTranscriptionPreferences({ getItem: () => "{invalid" })).toEqual(
      DEFAULT_TRANSCRIPTION_PREFERENCES,
    );
  });

  it("falls back to defaults for out-of-range threads", () => {
    const storage = {
      getItem: () =>
        JSON.stringify({
          version: 1,
          model: "base",
          device: "cpu",
          threads: 513,
        }),
    };

    expect(loadTranscriptionPreferences(storage)).toEqual(
      DEFAULT_TRANSCRIPTION_PREFERENCES,
    );
  });
});
