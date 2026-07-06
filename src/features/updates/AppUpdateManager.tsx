import { useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { Check, RefreshCw, Sparkles, X } from "lucide-react";
import { Button } from "../../components/Button";

const INITIAL_CHECK_DELAY_MS = 2500;
const UPDATE_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
const UPDATE_TIMEOUT_MS = 30000;
const COMPLETED_UPDATE_STORAGE_KEY = "rescue-codex-completed-update";
const LAST_SEEN_VERSION_STORAGE_KEY = "rescue-codex-last-seen-version";

type UpdateStatus = "available" | "downloading" | "ready" | "error";

type CompletedUpdate = {
  fromVersion: string;
  toVersion: string;
  body?: string;
  date?: string;
  completedAt: string;
};

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function normalizeNotes(body?: string) {
  const lines = (body ?? "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => line.replace(/^[-*]\s+/, ""));

  return lines.length > 0 ? lines.slice(0, 12) : ["本次更新包含体验优化和稳定性改进。"];
}

function formatPercent(downloaded: number, total: number) {
  if (total <= 0) {
    return "下载中";
  }

  return `${Math.min(100, Math.round((downloaded / total) * 100))}%`;
}

function readCompletedUpdate() {
  try {
    const raw = window.localStorage.getItem(COMPLETED_UPDATE_STORAGE_KEY);

    if (!raw) {
      return null;
    }

    return JSON.parse(raw) as CompletedUpdate;
  } catch {
    window.localStorage.removeItem(COMPLETED_UPDATE_STORAGE_KEY);
    return null;
  }
}

function storeCompletedUpdate(update: Update, currentVersion: string | null) {
  const completedUpdate: CompletedUpdate = {
    fromVersion: currentVersion ?? update.currentVersion,
    toVersion: update.version,
    body: update.body,
    date: update.date,
    completedAt: new Date().toISOString(),
  };

  window.localStorage.setItem(COMPLETED_UPDATE_STORAGE_KEY, JSON.stringify(completedUpdate));
}

function markVersionAsSeen(version: string) {
  window.localStorage.setItem(LAST_SEEN_VERSION_STORAGE_KEY, version);
}

export function AppUpdateManager() {
  const [visible, setVisible] = useState(false);
  const [status, setStatus] = useState<UpdateStatus>("available");
  const [update, setUpdate] = useState<Update | null>(null);
  const [completedUpdate, setCompletedUpdate] = useState<CompletedUpdate | null>(null);
  const [currentVersion, setCurrentVersion] = useState<string | null>(null);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [contentLength, setContentLength] = useState(0);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const checkingRef = useRef(false);

  const checkForUpdates = useCallback(async () => {
    if (!isTauriRuntime() || checkingRef.current || completedUpdate) {
      return;
    }

    checkingRef.current = true;

    try {
      const [foundUpdate, localVersion] = await Promise.all([
        check({ timeout: UPDATE_TIMEOUT_MS }),
        getVersion().catch(() => null),
      ]);

      if (!foundUpdate) {
        return;
      }

      setUpdate(foundUpdate);
      setCurrentVersion(localVersion ?? foundUpdate.currentVersion);
      setStatus("available");
      setErrorMessage(null);
      setDownloadedBytes(0);
      setContentLength(0);
      setVisible(true);
    } catch {
      // Silent background checks should not interrupt normal use.
    } finally {
      checkingRef.current = false;
    }
  }, [completedUpdate]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    void getVersion()
      .then((localVersion) => {
        const completed = readCompletedUpdate();

        if (completed && localVersion === completed.toVersion) {
          setCompletedUpdate(completed);
          setVisible(true);
          return;
        }

        const lastSeenVersion = window.localStorage.getItem(LAST_SEEN_VERSION_STORAGE_KEY);

        if (lastSeenVersion && lastSeenVersion !== localVersion) {
          setCompletedUpdate({
            fromVersion: lastSeenVersion,
            toVersion: localVersion,
            completedAt: new Date().toISOString(),
          });
          setVisible(true);
          return;
        }

        markVersionAsSeen(localVersion);
      })
      .catch(() => {
        window.localStorage.removeItem(COMPLETED_UPDATE_STORAGE_KEY);
      });
  }, []);

  useEffect(() => {
    const initialCheck = window.setTimeout(checkForUpdates, INITIAL_CHECK_DELAY_MS);
    const interval = window.setInterval(checkForUpdates, UPDATE_CHECK_INTERVAL_MS);

    return () => {
      window.clearTimeout(initialCheck);
      window.clearInterval(interval);
    };
  }, [checkForUpdates]);

  const installUpdate = async () => {
    if (!update || status === "downloading") {
      return;
    }

    setStatus("downloading");
    setErrorMessage(null);
    setDownloadedBytes(0);
    setContentLength(0);

    let receivedBytes = 0;
    let totalBytes = 0;

    try {
      await update.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? 0;
          setContentLength(totalBytes);
          setDownloadedBytes(0);
          return;
        }

        if (event.event === "Progress") {
          receivedBytes += event.data.chunkLength;
          setDownloadedBytes(receivedBytes);
          return;
        }

        if (event.event === "Finished") {
          setDownloadedBytes(totalBytes || receivedBytes);
        }
      });

      storeCompletedUpdate(update, currentVersion);
      setStatus("ready");
    } catch (error) {
      console.warn("Failed to install update", error);
      setStatus("error");
      setErrorMessage("更新失败，请稍后重试。");
    }
  };

  const restartApp = async () => {
    await relaunch();
  };

  const closeCompletedUpdate = () => {
    if (completedUpdate) {
      markVersionAsSeen(completedUpdate.toVersion);
    }
    window.localStorage.removeItem(COMPLETED_UPDATE_STORAGE_KEY);
    setCompletedUpdate(null);
    setVisible(false);
  };

  if (!visible || (!update && !completedUpdate)) {
    return null;
  }

  const activeVersion = completedUpdate?.toVersion ?? update?.version ?? "";
  const notes = normalizeNotes(completedUpdate?.body ?? update?.body);
  const percent = contentLength > 0 ? Math.min(100, Math.round((downloadedBytes / contentLength) * 100)) : 0;
  const progressStyle = { "--update-progress": `${percent}%` } as CSSProperties;
  const isDownloading = status === "downloading";
  const canClose = !isDownloading;
  const isCompleted = Boolean(completedUpdate);

  return (
    <div className="update-modal" role="dialog" aria-modal="true" aria-labelledby="update-modal-title">
      <div className="update-modal__card">
        <header className="update-modal__header">
          <div className={isCompleted ? "update-modal__icon update-modal__icon--success" : "update-modal__icon"} aria-hidden="true">
            <Sparkles className="h-6 w-6" />
          </div>
          <div>
            <h2 id="update-modal-title">{isCompleted ? "🎉 更新成功！" : "发现新版本"}</h2>
            {completedUpdate ? (
              <p>
                已从 v{completedUpdate.fromVersion} 更新到 v{completedUpdate.toVersion}
              </p>
            ) : (
              <p>
                当前版本 v{currentVersion ?? update?.currentVersion}，新版本 v{activeVersion} 已可用。
              </p>
            )}
          </div>
          <button
            type="button"
            className="update-modal__close"
            aria-label={isCompleted ? "关闭" : "稍后再说"}
            disabled={!canClose}
            onClick={isCompleted ? closeCompletedUpdate : () => setVisible(false)}
          >
            <X className="h-5 w-5" />
          </button>
        </header>

        <section className="update-modal__body">
          <div className="update-modal__version-row">
            <span>v{activeVersion}</span>
            {(completedUpdate?.date ?? update?.date) ? <time>{new Date(completedUpdate?.date ?? update?.date ?? "").toLocaleDateString()}</time> : null}
          </div>

          {isCompleted ? null : (
            <div className="update-progress" style={progressStyle}>
              <div className="update-progress__track" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent}>
                <span className={isDownloading && contentLength === 0 ? "is-indeterminate" : ""} />
              </div>
              <p>{isDownloading ? `下载中... ${formatPercent(downloadedBytes, contentLength)}` : status === "ready" ? "更新已安装，重启后生效。" : "准备好后即可下载并安装。"}</p>
            </div>
          )}

          <div className="update-modal__notes">
            <h3>更新内容</h3>
            <ul>
              {notes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          </div>

          {errorMessage ? <p className="update-modal__error">{errorMessage}</p> : null}
        </section>

        <footer className="update-modal__footer">
          {isCompleted ? (
            <Button icon={<Sparkles className="h-4 w-4" />} onClick={closeCompletedUpdate}>
              我知道了
            </Button>
          ) : status !== "ready" ? (
            <>
              <Button variant="secondary" disabled={!canClose} onClick={() => setVisible(false)}>
                稍后
              </Button>
              <Button icon={<RefreshCw className={isDownloading ? "h-4 w-4 animate-spin" : "h-4 w-4"} />} disabled={isDownloading} onClick={installUpdate}>
                {isDownloading ? "下载中..." : status === "error" ? "重新下载" : "下载并安装"}
              </Button>
            </>
          ) : (
            <>
              <Button variant="secondary" onClick={() => setVisible(false)}>
                稍后重启
              </Button>
              <Button icon={<Check className="h-4 w-4" />} onClick={restartApp}>
                重启应用
              </Button>
            </>
          )}
        </footer>
      </div>
    </div>
  );
}
