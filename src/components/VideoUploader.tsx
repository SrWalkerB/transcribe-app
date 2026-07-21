import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useLang } from "../LangContext";
import {
  loadTranscriptionPreferences,
  saveTranscriptionPreferences,
  type TranscriptionOptions,
} from "../transcriptionPreferences";

const ACCEPTED_EXTENSIONS = ["mp4", "mkv", "avi", "mov", "webm"];

const MODEL_KEYS = [
  { value: "tiny", label: "Tiny", descKey: "model.tiny" },
  { value: "base", label: "Base", descKey: "model.base" },
  { value: "small", label: "Small", descKey: "model.small" },
  { value: "medium", label: "Medium", descKey: "model.medium" },
  { value: "large", label: "Large", descKey: "model.large" },
  { value: "turbo", label: "Turbo", descKey: "model.turbo" },
] as const;

// Tamanho aproximado do arquivo GGML por modelo (download whisper.cpp no Windows).
const MODEL_SIZES: Record<string, string> = {
  tiny: "~75 MB",
  base: "~148 MB",
  small: "~488 MB",
  medium: "~1,5 GB",
  large: "~3,1 GB",
  turbo: "~1,6 GB",
};

const DEVICE_KEYS = [
  { value: "auto", labelKey: "upload.device.auto" },
  { value: "gpu", labelKey: "upload.device.gpu" },
  { value: "cpu", labelKey: "upload.device.cpu" },
] as const;

interface SelectedVideo {
  path: string | null;
  displayName: string;
  size?: number;
  duration?: number;
}

interface DroppedFile {
  path: string;
  displayName: string;
  duration?: number;
}

interface VideoUploaderProps {
  onPathSelected: (path: string, model: string, threads: number, device: string) => void;
  isLoading: boolean;
  droppedFile: DroppedFile | null;
  dragOver: boolean;
}

export default function VideoUploader({ onPathSelected, isLoading, droppedFile, dragOver }: VideoUploaderProps) {
  const { t } = useLang();
  const [selected, setSelected] = useState<SelectedVideo | null>(null);
  const [loadingFile, setLoadingFile] = useState(false);
  const [preferences, setPreferences] = useState(() =>
    loadTranscriptionPreferences(localStorage),
  );
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [maxThreads, setMaxThreads] = useState(4);
  const [isWindows, setIsWindows] = useState(false);
  const [modelReady, setModelReady] = useState(true);
  const [downloadingModel, setDownloadingModel] = useState(false);
  const [modelDownloadStep, setModelDownloadStep] = useState("");
  const [modelError, setModelError] = useState("");
  const { model, device, threads } = preferences;

  function updatePreferences(next: TranscriptionOptions) {
    const saved = { version: 1 as const, ...next };
    setPreferences(saved);
    saveTranscriptionPreferences(localStorage, saved);
  }

  useEffect(() => {
    invoke<number>("get_cpu_count").then((count) => {
      setMaxThreads(count);
      setPreferences((current) => {
        const threads = Math.min(current.threads, count);
        if (threads === current.threads) return current;
        const saved = { ...current, threads };
        saveTranscriptionPreferences(localStorage, saved);
        return saved;
      });
    });
    invoke<string>("get_platform")
      .then((p) => setIsWindows(p === "windows"))
      .catch(() => {});
  }, []);

  // No Windows (whisper.cpp), verifica se o modelo selecionado já foi baixado.
  useEffect(() => {
    if (!isWindows) {
      setModelReady(true);
      return;
    }
    let cancelled = false;
    setModelError("");
    invoke<boolean>("is_model_downloaded", { model })
      .then((ready) => {
        if (!cancelled) setModelReady(ready);
      })
      .catch(() => {
        if (!cancelled) setModelReady(false);
      });
    return () => {
      cancelled = true;
    };
  }, [isWindows, model]);

  // Progresso do download do modelo (evento install-progress, target "model").
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    listen<{ target: string; step: string }>("install-progress", (event) => {
      if (event.payload.target === "model") {
        setModelDownloadStep(event.payload.step);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  // When a file is dropped via Tauri native drag-drop
  useEffect(() => {
    if (droppedFile) {
      setSelected({
        path: droppedFile.path,
        displayName: droppedFile.displayName,
        duration: droppedFile.duration,
      });
    }
  }, [droppedFile]);

  async function handleBrowseClick() {
    const result = await open({
      multiple: false,
      filters: [{ name: "Video", extensions: ACCEPTED_EXTENSIONS }],
    });
    const path = result === null || Array.isArray(result) ? null : result;
    if (path) {
      const displayName = path.replace(/^.*[/\\]/, "") || "video";
      setLoadingFile(true);
      let duration: number | undefined;
      try {
        duration = await invoke<number>("get_video_duration", { path });
      } catch {
        // ffprobe not available, skip duration
      }
      setSelected({ path, displayName, duration });
      setLoadingFile(false);
    }
  }

  async function handleTranscribe() {
    if (!selected?.path) return;

    // No Windows, baixa o modelo GGML sob demanda antes de transcrever.
    if (isWindows && !modelReady) {
      setDownloadingModel(true);
      setModelError("");
      setModelDownloadStep("");
      try {
        await invoke<string>("download_model", { model });
        setModelReady(true);
      } catch (err) {
        setModelError(err instanceof Error ? err.message : String(err));
        setDownloadingModel(false);
        return;
      }
      setDownloadingModel(false);
    }

    onPathSelected(selected.path, model, threads, device);
  }

  function handleRemoveFile() {
    setSelected(null);
  }

  function formatDuration(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + " MB";
    return (bytes / (1024 * 1024 * 1024)).toFixed(2) + " GB";
  }

  return (
    <div className="uploader-wrapper">
      <div
        role="button"
        tabIndex={0}
        className={`drop-zone ${dragOver ? "drop-zone--active" : ""} ${selected ? "drop-zone--has-file" : ""}`}
        onClick={() => !selected && handleBrowseClick()}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            !selected && handleBrowseClick();
          }
        }}
      >
        {loadingFile ? (
          <div className="drop-zone__content">
            <span className="spinner" />
            <p className="drop-zone__label">{t("upload.loadingFile")}</p>
          </div>
        ) : !selected ? (
          <div className="drop-zone__content">
            <div className="drop-zone__icon">
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <title>Upload icon</title>
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="17 8 12 3 7 8" />
                <line x1="12" y1="3" x2="12" y2="15" />
              </svg>
            </div>
            <p className="drop-zone__label">
              {t("upload.dragOrBrowse")} <span className="drop-zone__browse">{t("upload.browse")}</span>
            </p>
            <p className="drop-zone__hint">
              {t("upload.formats")}
            </p>
          </div>
        ) : (
          <div className="drop-zone__file-info">
            <div className="file-icon">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <title>Video file icon</title>
                <polygon points="23 7 16 12 23 17 23 7" />
                <rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
              </svg>
            </div>
            <div className="file-details">
              <p className="file-name">{selected.displayName}</p>
              <p className="file-size">
                {selected.size != null && formatFileSize(selected.size)}
                {selected.size != null && selected.duration != null && " · "}
                {selected.duration != null && (
                  <span>{t("upload.duration")}: {formatDuration(selected.duration)}</span>
                )}
              </p>
            </div>
            <button
              type="button"
              className="file-remove"
              onClick={(e) => {
                e.stopPropagation();
                handleRemoveFile();
              }}
              title={t("upload.removeFile")}
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <title>Remove file</title>
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
        )}
      </div>

      {selected && (
        <>
          <div className="model-selector">
            <label className="model-label">{t("upload.modelLabel")}</label>
            <div className="model-options">
              {MODEL_KEYS.map((m) => (
                <button
                  key={m.value}
                  type="button"
                  className={`model-option ${model === m.value ? "model-option--active" : ""}`}
                  onClick={() => updatePreferences({ model: m.value, device, threads })}
                  disabled={isLoading}
                >
                  <span className="model-option__name">{m.label}</span>
                  <span className="model-option__desc">{t(m.descKey)}</span>
                </button>
              ))}
            </div>
          </div>

          <button
            type="button"
            className="btn-advanced-toggle"
            onClick={() => setShowAdvanced(!showAdvanced)}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
              className={`advanced-chevron ${showAdvanced ? "advanced-chevron--open" : ""}`}
            >
              <polyline points="6 9 12 15 18 9" />
            </svg>
            {t("upload.advanced")}
          </button>

          {showAdvanced && (
            <div className="advanced-options">
              {isWindows && (
                <div className="device-control">
                  <label className="model-label">{t("upload.deviceLabel")}</label>
                  <div className="model-options model-options--device">
                    {DEVICE_KEYS.map((d) => (
                      <button
                        key={d.value}
                        type="button"
                        className={`model-option ${device === d.value ? "model-option--active" : ""}`}
                        onClick={() => updatePreferences({ model, device: d.value, threads })}
                        disabled={isLoading}
                      >
                        <span className="model-option__name">{t(d.labelKey)}</span>
                      </button>
                    ))}
                  </div>
                  <p className="settings-dep__hint">{t("upload.deviceHint")}</p>
                </div>
              )}
              {/* CPU Threads só é relevante quando há processamento em CPU.
                  No modo GPU explícito a inferência roda na placa, então escondemos. */}
              {!(isWindows && device === "gpu") && (
                <div className="threads-control">
                  <div className="threads-header">
                    <label className="model-label">CPU Threads</label>
                    <span className="threads-value">{threads}</span>
                  </div>
                  <input
                    type="range"
                    min={1}
                    max={maxThreads}
                    value={threads}
                    onChange={(e) =>
                      updatePreferences({
                        model,
                        device,
                        threads: Number(e.target.value),
                      })
                    }
                    className="threads-slider"
                    disabled={isLoading}
                  />
                  <div className="threads-range">
                    <span>1</span>
                    <span>{maxThreads}</span>
                  </div>
                </div>
              )}
            </div>
          )}

          {isWindows && !modelReady && !downloadingModel && (
            <p className="settings-page__hint">
              {t("upload.modelDownloadHint")} ({MODEL_SIZES[model] ?? ""})
            </p>
          )}

          {modelError && (
            <div className="error-banner error-banner--compact">
              <p>{modelError}</p>
            </div>
          )}

          <button
            type="button"
            className="btn-transcribe"
            onClick={handleTranscribe}
            disabled={isLoading || downloadingModel}
          >
            {downloadingModel ? (
              <>
                <span className="spinner" />
                {modelDownloadStep || t("upload.downloadingModel")}
              </>
            ) : isLoading ? (
              <>
                <span className="spinner" />
                {t("upload.transcribing")}
              </>
            ) : isWindows && !modelReady ? (
              t("upload.downloadAndTranscribe")
            ) : (
              t("upload.transcribe")
            )}
          </button>
        </>
      )}
    </div>
  );
}
