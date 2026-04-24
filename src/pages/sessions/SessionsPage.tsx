import { useEffect, useState } from "react";
import { Download, FileJson2, FileSpreadsheet, FileText } from "lucide-react";
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
  const [loading, setLoading] = useState(true);

  async function load(nextFilters: SessionListFilters) {
    setLoading(true);
    setError(null);

    try {
      const payload = await listSessions(nextFilters);
      setResponse(payload);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "无法加载会话。");
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
      return;
    }

    await exportReport({
      kind: "sessions",
      format,
      path,
      filters,
    });
  }

  useEffect(() => {
    void load(initialFilters);
  }, []);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        eyebrow="Sessions"
        title="会话列表与详情"
        description="支持按关键词、工作目录和来源筛选。"
        actions={
          <>
            <Button variant="secondary" onClick={() => void load(filters)} icon={<Download className="h-4 w-4" />}>
              刷新
            </Button>
            <Button variant="secondary" onClick={() => void handleExport("csv")} icon={<FileSpreadsheet className="h-4 w-4" />}>
              导出 CSV
            </Button>
            <Button variant="secondary" onClick={() => void handleExport("json")} icon={<FileJson2 className="h-4 w-4" />}>
              导出 JSON
            </Button>
            <Button variant="secondary" onClick={() => void handleExport("markdown")} icon={<FileText className="h-4 w-4" />}>
              导出 Markdown
            </Button>
          </>
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
