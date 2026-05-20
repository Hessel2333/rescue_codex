import { useEffect, useState } from "react";
import { FileJson2, FileSpreadsheet, FileText, RefreshCw } from "lucide-react";
import clsx from "clsx";
import { Button } from "../../components/Button";
import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { SessionDetailPanel } from "../../features/sessions/SessionDetailPanel";
import { exportReport, listSessions, pickSavePath } from "../../lib/tauri";
import { formatDateTime, formatDuration, formatOptionalText } from "../../lib/format";
import { SessionListFilters, SessionListResponse, SessionSummary } from "../../types/api";

const initialFilters: SessionListFilters = {
  limit: 25,
  offset: 0,
};

function SessionListItem({
  item,
  selected,
  onSelect,
}: {
  item: SessionSummary;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button type="button" className={clsx("session-list-item", selected && "is-selected")} onClick={onSelect}>
      <div className="session-list-item__main">
        <p className="session-list-item__title">{formatOptionalText(item.threadTitle, "未命名会话")}</p>
        <p className="session-list-item__meta">{formatOptionalText(item.source, "未知来源")}</p>
      </div>
      <div className="session-list-item__stats">
        <span>{formatDateTime(item.updatedAt)}</span>
        <span>{formatDuration(item.durationSec)}</span>
        <span>{item.turnCount} turns</span>
      </div>
      <p className="session-list-item__path">{formatOptionalText(item.cwd)}</p>
    </button>
  );
}

export function SessionsPage() {
  const [filters, setFilters] = useState<SessionListFilters>(initialFilters);
  const [queryDraft, setQueryDraft] = useState("");
  const [cwdDraft, setCwdDraft] = useState("");
  const [sourceDraft, setSourceDraft] = useState("");
  const [response, setResponse] = useState<SessionListResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [exporting, setExporting] = useState<"csv" | "json" | "markdown" | null>(null);
  const [loading, setLoading] = useState(true);

  async function load(nextFilters: SessionListFilters, announce = true) {
    setLoading(true);
    setError(null);
    if (announce) {
      setActionMessage("正在重新载入会话列表…");
    }

    try {
      const payload = await listSessions(nextFilters);
      setResponse(payload);
      if (announce) {
        setActionMessage("已重新载入会话列表。");
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "无法加载会话。");
      if (announce) {
        setActionMessage(null);
      }
    } finally {
      setLoading(false);
    }
  }

  async function selectSession(sessionId: string) {
    const nextFilters = { ...filters, sessionId };
    setFilters(nextFilters);
    await load(nextFilters);
  }

  async function applyFilters() {
    const nextFilters: SessionListFilters = {
      ...filters,
      query: queryDraft || undefined,
      cwd: cwdDraft || undefined,
      source: sourceDraft || undefined,
      sessionId: undefined,
      offset: 0,
    };

    setFilters(nextFilters);
    await load(nextFilters);
  }

  async function handleExport(format: "csv" | "json" | "markdown") {
    const extension = format === "markdown" ? "md" : format;
    const path = await pickSavePath(`rescue_codex-sessions.${extension}`);
    if (!path) {
      setActionMessage("已取消导出。");
      return;
    }

    setExporting(format);
    setActionMessage(`正在导出 ${format === "markdown" ? "Markdown" : format.toUpperCase()}…`);

    try {
      const result = await exportReport({
        kind: "sessions",
        format,
        path,
        filters,
      });
      setActionMessage(`已导出：${result.path}`);
    } catch (cause) {
      setActionMessage(cause instanceof Error ? `导出失败：${cause.message}` : "导出失败。");
    } finally {
      setExporting(null);
    }
  }

  useEffect(() => {
    void load(initialFilters, false);
  }, []);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        eyebrow="Sessions"
        title="会话列表与详情"
        description="支持按关键词、工作目录和来源筛选。"
        actions={
          <div className="action-stack">
            <div className="action-row">
              <Button variant="secondary" disabled={loading} onClick={() => void load(filters)} icon={<RefreshCw className="h-4 w-4" />}>
                {loading ? "重新载入中" : "重新载入"}
              </Button>
              <Button
                variant="secondary"
                disabled={exporting !== null}
                onClick={() => void handleExport("csv")}
                icon={<FileSpreadsheet className="h-4 w-4" />}
              >
                {exporting === "csv" ? "导出中" : "导出 CSV"}
              </Button>
              <Button
                variant="secondary"
                disabled={exporting !== null}
                onClick={() => void handleExport("json")}
                icon={<FileJson2 className="h-4 w-4" />}
              >
                {exporting === "json" ? "导出中" : "导出 JSON"}
              </Button>
              <Button
                variant="secondary"
                disabled={exporting !== null}
                onClick={() => void handleExport("markdown")}
                icon={<FileText className="h-4 w-4" />}
              >
                {exporting === "markdown" ? "导出中" : "导出 Markdown"}
              </Button>
            </div>
            {actionMessage ? <p className="action-status">{actionMessage}</p> : null}
          </div>
        }
      />

      <Panel title="筛选器">
        <div className="grid gap-4 lg:grid-cols-5">
          <input
            value={queryDraft}
            onChange={(event) => setQueryDraft(event.target.value)}
            placeholder="关键词 / 标题 / 首条消息"
            className="field-input"
          />
          <input
            value={cwdDraft}
            onChange={(event) => setCwdDraft(event.target.value)}
            placeholder="cwd 包含"
            className="field-input"
          />
          <input
            value={sourceDraft}
            onChange={(event) => setSourceDraft(event.target.value)}
            placeholder="source 包含"
            className="field-input"
          />
          <input
            type="date"
            value={filters.dateFrom ?? ""}
            onChange={(event) => setFilters((current) => ({ ...current, dateFrom: event.target.value || undefined }))}
            className="field-input"
          />
          <div className="flex gap-3">
            <input
              type="date"
              value={filters.dateTo ?? ""}
              onChange={(event) => setFilters((current) => ({ ...current, dateTo: event.target.value || undefined }))}
              className="field-input min-w-0 flex-1"
            />
            <Button onClick={() => void applyFilters()}>应用</Button>
          </div>
        </div>
      </Panel>

      {error ? <Panel title="载入失败">{error}</Panel> : null}

      <div className="sessions-layout">
        <Panel title="会话列表" description={`当前共 ${response?.total ?? 0} 条会话。`} className="sessions-list-panel">
          {loading ? <div className="body-text">正在加载...</div> : null}
          <div className="sessions-list">
            {response?.items.map((item) => (
              <SessionListItem
                key={item.id}
                item={item}
                selected={response.selected?.session.id === item.id}
                onSelect={() => void selectSession(item.id)}
              />
            ))}
          </div>
        </Panel>

        <SessionDetailPanel detail={response?.selected} />
      </div>
    </div>
  );
}
