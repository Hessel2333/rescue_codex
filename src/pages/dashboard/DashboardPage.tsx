import { Download, FileJson2, FileText } from "lucide-react";
import { Button } from "../../components/Button";
import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { OverviewCharts } from "../../features/dashboard/ChartsPanel";
import { AnalyticsToolbar } from "../../features/dashboard/AnalyticsToolbar";
import { OverviewCards } from "../../features/dashboard/OverviewCards";
import { useDashboardControls } from "../../features/dashboard/dashboardControls";
import { useDashboardSummary } from "../../features/dashboard/useDashboardSummary";
import { formatDateTime, formatDuration, formatNumber, formatOptionalText } from "../../lib/format";
import { sanitizeMessagePreview } from "../../lib/sessionVisibility";
import { exportReport, pickSavePath } from "../../lib/tauri";

function AccountValue({ label, value }: { label: string; value?: string | null }) {
  return (
    <div className="surface-tile">
      <p className="meta-label">{label}</p>
      <p className="mono-value mt-2">{formatOptionalText(value)}</p>
    </div>
  );
}

export function DashboardPage() {
  const {
    timeUnit,
    setTimeUnit,
    anchorDate,
    setAnchorDate,
    granularity,
    setGranularity,
    project,
    setProject,
    filters,
  } = useDashboardControls();
  const { summary, loading, error, refresh } = useDashboardSummary(filters, "overview");

  async function handleExport(format: "json" | "markdown") {
    const path = await pickSavePath(`rescue_codex-dashboard.${format === "markdown" ? "md" : format}`);
    if (!path) {
      return;
    }

    await exportReport({
      kind: "dashboard",
      format,
      path,
      dashboardFilters: filters,
    });
  }

  const availableFrom = summary?.scope.availableFrom;
  const availableTo = summary?.scope.availableTo;
  const currentScopeText = summary ? `${summary.scope.dateFrom} 至 ${summary.scope.dateTo}` : "载入中";
  const previousDisabled =
    timeUnit === "all" || !availableFrom || (filters.dateFrom ? filters.dateFrom <= availableFrom : false);
  const nextDisabled =
    timeUnit === "all" || !availableTo || (filters.dateTo ? filters.dateTo >= availableTo : false);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        eyebrow="Overview"
        title="Codex 使用概览"
        description="活跃趋势与最近记录。"
        actions={
          <>
            <Button variant="secondary" onClick={refresh} icon={<Download className="h-4 w-4" />}>
              刷新数据
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

      <AnalyticsToolbar
        timeUnit={timeUnit}
        setTimeUnit={setTimeUnit}
        anchorDate={anchorDate}
        setAnchorDate={setAnchorDate}
        granularity={granularity}
        setGranularity={setGranularity}
        project={project}
        setProject={setProject}
        projectOptions={summary?.projectOptions ?? []}
        previousDisabled={previousDisabled}
        nextDisabled={nextDisabled}
      />

      {loading && !summary ? <Panel title="载入中">正在读取本地 SQLite、会话事件和日志数据…</Panel> : null}
      {error ? <Panel title="载入失败">{error}</Panel> : null}

      {summary ? (
        <>
          <OverviewCards overview={summary.overview} scope={summary.scope} />
          <OverviewCharts scope={summary.scope} activity={summary.activity} heatmapActivity={summary.heatmapActivity} />

          <div className="grid gap-6 xl:grid-cols-2">
            <Panel title="工作区分布" description="当前时间范围内，高频工作目录按会话数聚合。">
              <div className="stack-list">
                {summary.topCwds.map((item) => (
                  <div key={item.label} className="surface-row">
                    <span className="mono-value max-w-[80%]">{formatOptionalText(item.label)}</span>
                    <span className="metric-pill">{formatNumber(item.value)}</span>
                  </div>
                ))}
              </div>
            </Panel>

            <Panel title="账户与运行配置" description="从本机配置和认证状态读取的当前运行画像，敏感信息默认脱敏。">
              <div className="grid gap-4 md:grid-cols-2">
                <AccountValue label="账号邮箱" value={summary.accountInfo.maskedEmail} />
                <AccountValue label="套餐" value={summary.accountInfo.planType} />
                <AccountValue label="账号 ID" value={summary.accountInfo.maskedAccountUserId} />
                <AccountValue label="默认速度" value={summary.accountInfo.currentSpeedTier} />
                <AccountValue label="默认模型" value={summary.accountInfo.currentModel} />
                <AccountValue label="默认推理强度" value={summary.accountInfo.currentReasoningEffort} />
                <AccountValue label="最近刷新" value={summary.accountInfo.lastRefresh} />
                <AccountValue label="平台" value={summary.appInfo.platform} />
              </div>

              <div className="surface-block mt-5">
                <p className="meta-label">会话来源</p>
                <div className="mt-3 flex flex-wrap gap-2">
                  {summary.topSources.map((item) => (
                    <span key={item.label} className="metric-pill">
                      {item.label} · {formatNumber(item.value)}
                    </span>
                  ))}
                </div>
              </div>
            </Panel>
          </div>

          <div className="grid gap-6 xl:grid-cols-2">
            <Panel title="最近导入" description="最近一次扫描或手动导入的执行情况。">
              <div className="overflow-x-auto">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>来源</th>
                      <th>状态</th>
                      <th>文件</th>
                      <th>完成时间</th>
                    </tr>
                  </thead>
                  <tbody>
                    {summary.recentImports.map((item) => (
                      <tr key={item.id}>
                        <td>{item.sourceLabel}</td>
                        <td>{item.status}</td>
                        <td>{`${item.filesSuccess}/${item.filesTotal}`}</td>
                        <td>{formatDateTime(item.finishedAt ?? item.startedAt)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Panel>

            <Panel title="最近告警" description="坏行、缺失字段和格式漂移会记录在这里。">
              <div className="stack-list">
                {summary.recentIssues.length === 0 ? (
                  <div className="empty-state">目前没有告警记录。</div>
                ) : null}
                {summary.recentIssues.map((issue) => (
                  <div key={issue.id} className="warning-card">
                    <div className="warning-card__meta">
                      <span>{issue.severity}</span>
                      <span>{issue.code}</span>
                      <span>{formatDateTime(issue.createdAt)}</span>
                    </div>
                    <p className="body-text mt-2">{issue.message}</p>
                  </div>
                ))}
              </div>
            </Panel>
          </div>

          <Panel title="最近会话" description="保留会话级摘要，完整时间线仍可在 Sessions 页面查看。">
            <div className="stack-list">
              {summary.recentSessions.map((session) => (
                <div key={session.id} className="surface-block">
                  <div className="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
                    <div>
                      <p className="section-title">{formatOptionalText(session.threadTitle, "未命名会话")}</p>
                      <p className="mono-value mt-1 text-xs">{formatOptionalText(session.cwd)}</p>
                    </div>
                    <div className="flex flex-wrap gap-2 text-xs uppercase tracking-[0.14em]">
                      <span className="metric-pill">{formatDuration(session.durationSec)}</span>
                      <span className="metric-pill">{session.turnCount} turns</span>
                      <span className="metric-pill">{session.toolCallCount} tools</span>
                    </div>
                  </div>
                  <p className="body-text mt-3">
                    {formatOptionalText(sanitizeMessagePreview(session.firstUserMessage), "暂无首条用户消息")}
                  </p>
                </div>
              ))}
            </div>
          </Panel>
        </>
      ) : null}
    </div>
  );
}
