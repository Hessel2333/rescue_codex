import { useEffect, useRef, useState } from "react";
import { getDashboardSummary, scanDefaultSource } from "../../lib/tauri";
import { notifyImportDataRefresh } from "./importEvents";

const enabledStorageKey = "rescue_codex.autoImport.enabled";
const intervalStorageKey = "rescue_codex.autoImport.intervalMinutes";
const settingsChangedEvent = "rescue_codex:auto-import-settings-changed";

export const defaultAutoImportIntervalMinutes = 15;
export const autoImportIntervalOptions = [5, 15, 30, 60] as const;

export type AutoImportSyncSettings = {
  enabled: boolean;
  intervalMinutes: number;
};

function parseInterval(value: string | null) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return defaultAutoImportIntervalMinutes;
  }

  return Math.min(Math.max(Math.round(parsed), 5), 24 * 60);
}

export function readAutoImportSyncSettings(): AutoImportSyncSettings {
  if (typeof window === "undefined") {
    return {
      enabled: true,
      intervalMinutes: defaultAutoImportIntervalMinutes,
    };
  }

  return {
    enabled: window.localStorage.getItem(enabledStorageKey) !== "false",
    intervalMinutes: parseInterval(window.localStorage.getItem(intervalStorageKey)),
  };
}

export function saveAutoImportSyncSettings(settings: AutoImportSyncSettings) {
  if (typeof window === "undefined") {
    return;
  }

  window.localStorage.setItem(enabledStorageKey, String(settings.enabled));
  window.localStorage.setItem(intervalStorageKey, String(settings.intervalMinutes));
  window.dispatchEvent(new Event(settingsChangedEvent));
}

function isAlreadyRunningError(cause: unknown) {
  const message = cause instanceof Error ? cause.message : String(cause);
  return message.includes("已有导入任务正在运行");
}

async function waitForImportCompletion(cancelled: () => boolean) {
  while (!cancelled()) {
    const summary = await getDashboardSummary({}, "imports");
    notifyImportDataRefresh();

    const importRunning = summary.recentImports.some((item) => item.status === "running");
    if (!importRunning) {
      return;
    }

    await new Promise((resolve) => window.setTimeout(resolve, 3000));
  }
}

export function useAutoImportSync() {
  const [settings, setSettings] = useState(readAutoImportSyncSettings);
  const runningRef = useRef(false);

  useEffect(() => {
    const sync = () => setSettings(readAutoImportSyncSettings());

    window.addEventListener(settingsChangedEvent, sync);
    window.addEventListener("storage", sync);
    return () => {
      window.removeEventListener(settingsChangedEvent, sync);
      window.removeEventListener("storage", sync);
    };
  }, []);

  useEffect(() => {
    if (!settings.enabled) {
      return undefined;
    }

    let cancelled = false;

    async function startSync() {
      if (cancelled || runningRef.current) {
        return;
      }

      runningRef.current = true;
      try {
        const result = await scanDefaultSource();
        notifyImportDataRefresh();

        if (result.status === "running") {
          await waitForImportCompletion(() => cancelled);
        }
      } catch (cause) {
        if (!isAlreadyRunningError(cause)) {
          console.warn("Auto import failed", cause);
        }
      } finally {
        runningRef.current = false;
      }
    }

    const startupTimer = window.setTimeout(() => {
      void startSync();
    }, 1200);
    const intervalTimer = window.setInterval(
      () => {
        void startSync();
      },
      settings.intervalMinutes * 60 * 1000,
    );

    return () => {
      cancelled = true;
      window.clearTimeout(startupTimer);
      window.clearInterval(intervalTimer);
    };
  }, [settings.enabled, settings.intervalMinutes]);
}
