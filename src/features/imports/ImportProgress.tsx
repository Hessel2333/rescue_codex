import type { CSSProperties } from "react";
import { AlertTriangle, CheckCircle2, Loader2, XCircle } from "lucide-react";
import type { ImportRunResult, RecentImport } from "../../types/api";

type ImportProgressLike = Pick<
  RecentImport | ImportRunResult,
  "status" | "filesTotal" | "filesSuccess" | "filesFailed" | "warningsCount" | "errorsCount"
>;

function getImportProgress(item: ImportProgressLike) {
  const total = Math.max(0, item.filesTotal);
  const processed = Math.min(total, Math.max(0, item.filesSuccess + item.filesFailed));
  const percent = total > 0 ? Math.round((processed / total) * 100) : item.status === "completed" ? 100 : 0;

  return {
    total,
    processed,
    percent,
    isRunning: item.status === "running",
    hasErrors: item.errorsCount > 0 || item.filesFailed > 0 || item.status === "failed",
    hasWarnings: item.warningsCount > 0,
  };
}

function getStatusLabel(status: string) {
  switch (status) {
    case "running":
      return "导入中";
    case "completed":
      return "已完成";
    case "completed_with_warnings":
      return "有告警";
    case "failed":
      return "失败";
    case "interrupted":
      return "已中断";
    default:
      return status;
  }
}

export function ImportStatusBadge({ status }: { status: string }) {
  const Icon =
    status === "running"
      ? Loader2
      : status === "failed" || status === "interrupted"
        ? XCircle
        : status === "completed_with_warnings"
          ? AlertTriangle
          : CheckCircle2;

  return (
    <span className={`import-status import-status--${status}`}>
      <Icon className="h-4 w-4" />
      {getStatusLabel(status)}
    </span>
  );
}

export function ImportProgress({ item, compact = false }: { item: ImportProgressLike; compact?: boolean }) {
  const progress = getImportProgress(item);
  const style = { "--import-progress": `${progress.percent}%` } as CSSProperties;
  const tone = progress.hasErrors ? "error" : progress.hasWarnings ? "warning" : "normal";

  return (
    <div className={`import-progress import-progress--${tone} ${compact ? "import-progress--compact" : ""}`}>
      <div className="import-progress__meta">
        <ImportStatusBadge status={item.status} />
        <span className="import-progress__percent">{progress.total > 0 ? `${progress.percent}%` : "准备中"}</span>
      </div>
      <div
        className={`import-progress__track ${progress.isRunning && progress.total === 0 ? "is-indeterminate" : ""}`}
        aria-label="导入进度"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={progress.percent}
        role="progressbar"
      >
        <span className={`import-progress__bar ${progress.isRunning ? "is-running" : ""}`} style={style} />
      </div>
      {!compact ? (
        <div className="import-progress__detail">
          <span>{progress.total > 0 ? `已处理 ${progress.processed} 个文件` : "正在扫描可导入文件"}</span>
          {item.warningsCount > 0 ? <span>{item.warningsCount} 个告警</span> : null}
          {item.errorsCount > 0 || item.filesFailed > 0 ? (
            <span>{item.errorsCount + item.filesFailed} 个错误或失败</span>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
