import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { LANGUAGES } from "../i18n";
import { useLang } from "../LangContext";

type UpdateStatus =
  | "idle"
  | "checking"
  | "upToDate"
  | "available"
  | "downloading"
  | "installing"
  | "error";

interface DependencyStatus {
  ffmpeg: boolean;
  python: boolean;
  faster_whisper: boolean;
}

interface SettingsPageProps {
  onContinue: () => void;
  isFirstRun: boolean;
}

type Platform = "windows" | "macos" | "linux";

export default function SettingsPage({ onContinue, isFirstRun }: SettingsPageProps) {
  const { lang, setLang, t } = useLang();
  const [deps, setDeps] = useState<DependencyStatus | null>(null);
  const [checking, setChecking] = useState(true);
  const [installingTarget, setInstallingTarget] = useState<
    "ffmpeg" | "whisper" | "python" | null
  >(null);
  const [installMsg, setInstallMsg] = useState("");
  const [installError, setInstallError] = useState("");
  const [platform, setPlatform] = useState<Platform>("linux");
  const [currentVersion, setCurrentVersion] = useState<string>("");
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>("idle");
  const [availableUpdate, setAvailableUpdate] = useState<Update | null>(null);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [updateError, setUpdateError] = useState("");

  useEffect(() => {
    checkDeps();
    invoke<string>("get_platform")
      .then((p) => {
        if (p === "windows" || p === "macos" || p === "linux") {
          setPlatform(p);
        }
      })
      .catch(() => {});
    getVersion()
      .then(setCurrentVersion)
      .catch(() => {});
    checkForUpdate();
  }, []);

  async function checkForUpdate() {
    setUpdateStatus("checking");
    setUpdateError("");
    try {
      const update = await check();
      if (update) {
        setAvailableUpdate(update);
        setUpdateStatus("available");
      } else {
        setAvailableUpdate(null);
        setUpdateStatus("upToDate");
      }
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err));
      setUpdateStatus("error");
    }
  }

  async function handleInstallUpdate() {
    if (!availableUpdate) return;
    setUpdateStatus("downloading");
    setUpdateError("");
    setDownloadedBytes(0);
    setTotalBytes(0);
    try {
      await availableUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") {
          setTotalBytes(event.data.contentLength ?? 0);
        } else if (event.event === "Progress") {
          setDownloadedBytes((prev) => prev + event.data.chunkLength);
        } else if (event.event === "Finished") {
          setUpdateStatus("installing");
        }
      });
      await relaunch();
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err));
      setUpdateStatus("error");
    }
  }

  async function checkDeps() {
    setChecking(true);
    try {
      const status = await invoke<DependencyStatus>("check_dependencies");
      setDeps(status);
    } catch {
      // ignore
    } finally {
      setChecking(false);
    }
  }

  async function handleInstallFasterWhisper() {
    setInstallingTarget("whisper");
    setInstallMsg("");
    setInstallError("");
    try {
      const result = await invoke<string>("install_dependencies");
      setInstallMsg(result);
      setTimeout(() => checkDeps(), 1500);
    } catch (err) {
      setInstallError(err instanceof Error ? err.message : String(err));
    } finally {
      setInstallingTarget(null);
    }
  }

  async function handleInstallFfmpeg() {
    setInstallingTarget("ffmpeg");
    setInstallMsg("");
    setInstallError("");
    try {
      const result = await invoke<string>("install_ffmpeg");
      setInstallMsg(result);
      setTimeout(() => checkDeps(), 1500);
    } catch (err) {
      setInstallError(err instanceof Error ? err.message : String(err));
    } finally {
      setInstallingTarget(null);
    }
  }

  async function handleInstallPython() {
    setInstallingTarget("python");
    setInstallMsg("");
    setInstallError("");
    try {
      const result = await invoke<string>("install_python");
      setInstallMsg(result);
      setTimeout(() => checkDeps(), 1500);
    } catch (err) {
      setInstallError(err instanceof Error ? err.message : String(err));
    } finally {
      setInstallingTarget(null);
    }
  }

  const allOk = deps?.ffmpeg && deps?.python && deps?.faster_whisper;

  return (
    <div className="settings-page">
      {!isFirstRun && (
        <button type="button" className="settings-page__back" onClick={onContinue}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="15 18 9 12 15 6" />
          </svg>
          {t("history.back")}
        </button>
      )}

      <h2 className="settings-page__title">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
        {t("settings.title")}
      </h2>

      {/* Language section */}
      <div className="settings-page__section">
        <label className="settings-page__label">{t("settings.language")}</label>
        <div className="settings-page__lang-grid">
          {LANGUAGES.map((l) => (
            <button
              key={l.value}
              type="button"
              className={`settings-page__lang-btn ${lang === l.value ? "settings-page__lang-btn--active" : ""}`}
              onClick={() => setLang(l.value)}
            >
              {l.label}
            </button>
          ))}
        </div>
      </div>

      {/* Dependencies section */}
      <div className="settings-page__section">
        <div className="settings-page__section-header">
          <label className="settings-page__label">{t("settings.deps")}</label>
          <button
            type="button"
            className="settings-page__recheck"
            onClick={checkDeps}
            disabled={checking}
          >
            {checking ? (
              <span className="spinner spinner--small" />
            ) : (
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="1 4 1 10 7 10" />
                <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
              </svg>
            )}
            {t("deps.recheck")}
          </button>
        </div>

        {checking && !deps ? (
          <div className="settings-page__checking">
            <span className="spinner" />
            <span>{t("deps.checking")}</span>
          </div>
        ) : deps ? (
          <div className="settings-page__deps-list">
            <DepItem
              name="FFmpeg"
              ok={deps.ffmpeg}
              hint={!deps.ffmpeg ? t("deps.ffmpeg.hint") : undefined}
              canInstall={!deps.ffmpeg && deps.python}
              installing={installingTarget === "ffmpeg"}
              onInstall={handleInstallFfmpeg}
              installLabel={installingTarget === "ffmpeg" ? t("deps.ffmpeg.installing") : t("deps.ffmpeg.install")}
            />
            <DepItem
              name="Python 3"
              ok={deps.python}
              hint={!deps.python ? t(`deps.python.hint.${platform}`) : undefined}
              canInstall={!deps.python && platform === "windows"}
              installing={installingTarget === "python"}
              onInstall={handleInstallPython}
              installLabel={installingTarget === "python" ? t("deps.python.installing") : t("deps.python.install")}
            />
            <DepItem
              name="faster-whisper"
              ok={deps.faster_whisper}
              hint={
                !deps.faster_whisper && deps.python
                  ? t("deps.whisper.hint")
                  : !deps.faster_whisper && !deps.python
                    ? t("deps.whisper.needPython")
                    : undefined
              }
              canInstall={!deps.faster_whisper && deps.python}
              installing={installingTarget === "whisper"}
              onInstall={handleInstallFasterWhisper}
              installLabel={installingTarget === "whisper" ? t("deps.installing") : t("deps.install")}
            />
          </div>
        ) : null}

        {installMsg && (
          <div className="deps-success">
            <p>{installMsg}</p>
          </div>
        )}

        {installError && (
          <div className="error-banner error-banner--compact">
            <p>{installError}</p>
          </div>
        )}
      </div>

      {/* Updates section */}
      <div className="settings-page__section">
        <div className="settings-page__section-header">
          <label className="settings-page__label">{t("updates.title")}</label>
          <button
            type="button"
            className="settings-page__recheck"
            onClick={checkForUpdate}
            disabled={updateStatus === "checking" || updateStatus === "downloading" || updateStatus === "installing"}
          >
            {updateStatus === "checking" ? (
              <span className="spinner spinner--small" />
            ) : (
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="1 4 1 10 7 10" />
                <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
              </svg>
            )}
            {t("updates.check")}
          </button>
        </div>

        <div className="settings-dep">
          <div className="settings-dep__info">
            <span className="settings-dep__name">
              {t("updates.current")}: {currentVersion || "—"}
            </span>
            {updateStatus === "checking" && (
              <span className="settings-dep__hint">{t("updates.checking")}</span>
            )}
            {updateStatus === "upToDate" && (
              <span className="settings-dep__hint">{t("updates.upToDate")}</span>
            )}
            {updateStatus === "available" && availableUpdate && (
              <span className="settings-dep__hint">
                {t("updates.available")}: {availableUpdate.version}
              </span>
            )}
            {updateStatus === "downloading" && (
              <span className="settings-dep__hint">
                {t("updates.downloading")}
                {totalBytes > 0
                  ? ` ${Math.round((downloadedBytes / totalBytes) * 100)}%`
                  : ""}
              </span>
            )}
            {updateStatus === "installing" && (
              <span className="settings-dep__hint">{t("updates.installing")}</span>
            )}
            {updateStatus === "error" && updateError && (
              <span className="settings-dep__hint">{updateError}</span>
            )}
          </div>
          {updateStatus === "available" && (
            <button
              type="button"
              className="settings-dep__install"
              onClick={handleInstallUpdate}
            >
              {t("updates.install")}
            </button>
          )}
          {(updateStatus === "downloading" || updateStatus === "installing") && (
            <button type="button" className="settings-dep__install" disabled>
              <span className="spinner spinner--small" />
              {updateStatus === "downloading"
                ? t("updates.downloading")
                : t("updates.installing")}
            </button>
          )}
        </div>

        {updateStatus === "available" && (
          <p className="settings-page__hint">{t("updates.restartHint")}</p>
        )}
      </div>

      {/* Continue button */}
      <button
        type="button"
        className="btn-transcribe settings-page__continue"
        onClick={onContinue}
        disabled={!allOk}
      >
        {isFirstRun ? t("settings.start") : t("settings.save")}
      </button>

      {!allOk && (
        <p className="settings-page__hint">{t("settings.depsRequired")}</p>
      )}
    </div>
  );
}

function DepItem({
  name,
  ok,
  hint,
  canInstall,
  installing,
  onInstall,
  installLabel,
}: {
  name: string;
  ok: boolean;
  hint?: string;
  canInstall?: boolean;
  installing?: boolean;
  onInstall?: () => void;
  installLabel?: string;
}) {
  return (
    <div className={`settings-dep ${ok ? "settings-dep--ok" : "settings-dep--missing"}`}>
      <div className="settings-dep__status">
        {ok ? (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="settings-dep__icon--ok">
            <polyline points="20 6 9 17 4 12" />
          </svg>
        ) : (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" className="settings-dep__icon--missing">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        )}
      </div>
      <div className="settings-dep__info">
        <span className="settings-dep__name">{name}</span>
        {hint && <span className="settings-dep__hint">{hint}</span>}
      </div>
      {canInstall && onInstall && (
        <button
          type="button"
          className="settings-dep__install"
          onClick={onInstall}
          disabled={installing}
        >
          {installing && <span className="spinner spinner--small" />}
          {installLabel}
        </button>
      )}
    </div>
  );
}
