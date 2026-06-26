export const importDataRefreshEvent = "rescue_codex:import-data-refresh";

export function notifyImportDataRefresh() {
  if (typeof window === "undefined") {
    return;
  }

  window.dispatchEvent(new Event(importDataRefreshEvent));
}
