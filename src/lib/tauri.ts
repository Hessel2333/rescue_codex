import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  DashboardFilters,
  DashboardSection,
  DashboardSummary,
  ExportRequest,
  ExportResult,
  ImportRunResult,
  SessionListFilters,
  SessionListResponse,
} from "../types/api";

export async function scanDefaultSource() {
  return invoke<ImportRunResult>("scan_default_source");
}

export async function importPaths(paths: string[]) {
  return invoke<ImportRunResult>("import_paths", { paths });
}

export async function getDashboardSummary(filters: DashboardFilters = {}, section: DashboardSection = "all") {
  return invoke<DashboardSummary>("get_dashboard_summary", { filters, section });
}

export async function listSessions(filters: SessionListFilters = {}) {
  return invoke<SessionListResponse>("list_sessions", { filters });
}

export async function exportReport(request: ExportRequest) {
  return invoke<ExportResult>("export_report", { request });
}

export async function pickFiles() {
  const selected = await open({
    multiple: true,
    filters: [
      {
        name: "JSON / JSONL",
        extensions: ["json", "jsonl"],
      },
    ],
  });

  if (!selected) {
    return [];
  }

  return Array.isArray(selected) ? selected : [selected];
}

export async function pickFolders() {
  const selected = await open({
    multiple: true,
    directory: true,
  });

  if (!selected) {
    return [];
  }

  return Array.isArray(selected) ? selected : [selected];
}

export async function pickSavePath(defaultPath: string) {
  const selected = await save({
    defaultPath,
  });

  return selected ?? null;
}
