import { describe, expect, it } from "vitest";
import { decideStartupState } from "./startup";

describe("decideStartupState", () => {
  const windowsReady = {
    ffmpeg: true,
    python: false,
    faster_whisper: false,
    whisper_cli: true,
    model: true,
  };

  it("opens the uploader after completed setup with ready dependencies", () => {
    expect(decideStartupState(true, "windows", windowsReady)).toBe("idle");
  });

  it("opens settings on the first run", () => {
    expect(decideStartupState(false, "windows", windowsReady)).toBe("settings");
  });

  it("opens settings when a dependency is missing", () => {
    expect(
      decideStartupState(true, "windows", { ...windowsReady, model: false }),
    ).toBe("settings");
  });
});
