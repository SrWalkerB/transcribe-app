import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useLang } from "../LangContext";

interface DependencyStatus {
  ffmpeg: boolean;
  python: boolean;
  faster_whisper: boolean;
}

interface DependencyCheckProps {
  deps: DependencyStatus;
  onRecheck: () => void;
}

export default function DependencyCheck({ deps, onRecheck }: DependencyCheckProps) {
  const { t } = useLang();
  const [installingTarget, setInstallingTarget] = useState<"ffmpeg" | "whisper" | null>(null);
  const [installMsg, setInstallMsg] = useState("");
  const [installError, setInstallError] = useState("");

  async function handleInstallFfmpeg() {
    setInstallingTarget("ffmpeg");
    setInstallMsg("");
    setInstallError("");
    try {
      const result = await invoke<string>("install_ffmpeg");
      setInstallMsg(result);
      setTimeout(() => onRecheck(), 1500);
    } catch (err) {
      setInstallError(err instanceof Error ? err.message : String(err));
    } finally {
      setInstallingTarget(null);
    }
  }

  async function handleInstallFasterWhisper() {
    setInstallingTarget("whisper");
    setInstallMsg("");
    setInstallError("");
    try {
      const result = await invoke<string>("install_dependencies");
      setInstallMsg(result);
      setTimeout(() => onRecheck(), 1500);
    } catch (err) {
      setInstallError(err instanceof Error ? err.message : String(err));
    } finally {
      setInstallingTarget(null);
    }
  }

  return (
    <div className="deps-wrapper">
      <h2 className="deps-title">{t("deps.title")}</h2>
      <p className="deps-subtitle">{t("deps.subtitle")}</p>

      <div className="deps-list">
        <div className={`deps-item ${deps.ffmpeg ? "deps-item--ok" : "deps-item--missing"}`}>
          <span className="deps-icon">{deps.ffmpeg ? "\u2713" : "\u2717"}</span>
          <div className="deps-info">
            <span className="deps-name">FFmpeg</span>
            {!deps.ffmpeg && (
              <span className="deps-hint">{t("deps.ffmpeg.hint")}</span>
            )}
          </div>
          {!deps.ffmpeg && deps.python && (
            <button
              type="button"
              className="settings-dep__install"
              onClick={handleInstallFfmpeg}
              disabled={installingTarget !== null}
            >
              {installingTarget === "ffmpeg" ? (
                <>
                  <span className="spinner spinner--small" />
                  {t("deps.ffmpeg.installing")}
                </>
              ) : (
                t("deps.ffmpeg.install")
              )}
            </button>
          )}
        </div>

        <div className={`deps-item ${deps.python ? "deps-item--ok" : "deps-item--missing"}`}>
          <span className="deps-icon">{deps.python ? "\u2713" : "\u2717"}</span>
          <div className="deps-info">
            <span className="deps-name">Python 3</span>
            {!deps.python && (
              <span className="deps-hint">{t("deps.python.hint.linux")}</span>
            )}
          </div>
        </div>

        <div className={`deps-item ${deps.faster_whisper ? "deps-item--ok" : "deps-item--missing"}`}>
          <span className="deps-icon">{deps.faster_whisper ? "\u2713" : "\u2717"}</span>
          <div className="deps-info">
            <span className="deps-name">faster-whisper</span>
            {!deps.faster_whisper && deps.python && (
              <span className="deps-hint">{t("deps.whisper.hint")}</span>
            )}
            {!deps.faster_whisper && !deps.python && (
              <span className="deps-hint">{t("deps.whisper.needPython")}</span>
            )}
          </div>
        </div>
      </div>

      {installMsg && (
        <div className="deps-success">
          <p>{installMsg}</p>
        </div>
      )}

      {installError && (
        <div className="error-banner">
          <p>{installError}</p>
        </div>
      )}

      <div className="deps-actions">
        {!deps.faster_whisper && deps.python && (
          <button
            type="button"
            className="btn-transcribe"
            onClick={handleInstallFasterWhisper}
            disabled={installingTarget !== null}
          >
            {installingTarget === "whisper" ? (
              <>
                <span className="spinner" />
                {t("deps.installing")}
              </>
            ) : (
              t("deps.install")
            )}
          </button>
        )}

        <button
          type="button"
          className="btn-new"
          onClick={onRecheck}
          disabled={installingTarget !== null}
        >
          {t("deps.recheck")}
        </button>
      </div>
    </div>
  );
}
