import { FileJson2, FileText, RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "../../components/Button";
import { exportReport, pickSavePath } from "../../lib/tauri";
import type { DashboardFilters, ExportFormat } from "../../types/api";

type DashboardHeaderActionsProps = {
  baseName: string;
  filters: DashboardFilters;
  loading: boolean;
  onRefresh: () => void;
};

function exportExtension(format: ExportFormat) {
  return format === "markdown" ? "md" : format;
}

function exportLabel(format: ExportFormat) {
  return format === "markdown" ? "Markdown" : format.toUpperCase();
}

export function DashboardHeaderActions({ baseName, filters, loading, onRefresh }: DashboardHeaderActionsProps) {
  const [exporting, setExporting] = useState<ExportFormat | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const refreshRequested = useRef(false);

  useEffect(() => {
    if (!refreshRequested.current || loading) {
      return;
    }

    refreshRequested.current = false;
    setMessage("已重新载入当前页面数据。");
  }, [loading]);

  function handleRefresh() {
    refreshRequested.current = true;
    setMessage("正在重新载入当前页面数据…");
    onRefresh();
  }

  async function handleExport(format: Extract<ExportFormat, "json" | "markdown">) {
    const path = await pickSavePath(`rescue_codex-${baseName}.${exportExtension(format)}`);
    if (!path) {
      setMessage("已取消导出。");
      return;
    }

    setExporting(format);
    setMessage(`正在导出 ${exportLabel(format)}…`);

    try {
      const result = await exportReport({
        kind: "dashboard",
        format,
        path,
        dashboardFilters: filters,
      });
      setMessage(`已导出 ${exportLabel(format)}：${result.path}`);
    } catch (cause) {
      setMessage(cause instanceof Error ? `导出失败：${cause.message}` : "导出失败。");
    } finally {
      setExporting(null);
    }
  }

  return (
    <div className="action-stack">
      <div className="action-row">
        <Button variant="secondary" onClick={handleRefresh} disabled={loading} icon={<RefreshCw className="h-4 w-4" />}>
          {loading && refreshRequested.current ? "重新载入中" : "重新载入"}
        </Button>
        <Button
          variant="secondary"
          onClick={() => void handleExport("json")}
          disabled={exporting !== null}
          icon={<FileJson2 className="h-4 w-4" />}
        >
          {exporting === "json" ? "导出中" : "导出 JSON"}
        </Button>
        <Button
          variant="secondary"
          onClick={() => void handleExport("markdown")}
          disabled={exporting !== null}
          icon={<FileText className="h-4 w-4" />}
        >
          {exporting === "markdown" ? "导出中" : "导出 Markdown"}
        </Button>
      </div>
      {message ? <p className="action-status">{message}</p> : null}
    </div>
  );
}
