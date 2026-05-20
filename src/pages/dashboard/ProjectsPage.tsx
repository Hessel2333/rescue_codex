import type { EChartsOption } from "echarts";
import { useMemo } from "react";
import { useTheme } from "../../app/theme";
import { chartFontFamily, EChart } from "../../components/EChart";
import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { AnalyticsToolbar } from "../../features/dashboard/AnalyticsToolbar";
import { DashboardHeaderActions } from "../../features/dashboard/DashboardHeaderActions";
import { buildCategoryAxisLabel, categoryGridBottom, formatTimelineAxisLabel } from "../../features/dashboard/chartAxis";
import { useDashboardControls } from "../../features/dashboard/dashboardControls";
import { useDashboardSummary } from "../../features/dashboard/useDashboardSummary";
import { formatDateTime, formatDuration, formatNumber, formatOptionalText } from "../../lib/format";

function useChartColors() {
  const { resolvedTheme } = useTheme();

  return useMemo(() => {
    const styles = getComputedStyle(document.documentElement);
    const read = (name: string, fallback: string) => styles.getPropertyValue(name).trim() || fallback;
    return {
      axis: read("--chart-axis", resolvedTheme === "dark" ? "#94a3b8" : "#64748b"),
      grid: read("--chart-grid", resolvedTheme === "dark" ? "rgba(255,255,255,0.08)" : "rgba(15,23,42,0.08)"),
      tooltipBg: read("--chart-tooltip", resolvedTheme === "dark" ? "rgba(11,16,24,0.96)" : "rgba(255,255,255,0.96)"),
      tooltipText: read("--chart-tooltip-text", resolvedTheme === "dark" ? "#f8fafc" : "#0f172a"),
      accentA: read("--chart-accent-a", "#34d399"),
      accentB: read("--chart-accent-b", "#22d3ee"),
      accentC: read("--chart-accent-c", "#f59e0b"),
      accentD: read("--chart-accent-d", "#f97316"),
      accentE: read("--chart-accent-e", "#60a5fa"),
      accentF: read("--chart-accent-f", "#f472b6"),
    };
  }, [resolvedTheme]);
}

function buildEmptyGraphic(text: string, color: string) {
  return [
    {
      type: "text",
      left: "center",
      top: "middle",
      style: {
        text,
        fill: color,
        fontFamily: chartFontFamily,
        fontSize: 13,
        align: "center",
      },
    },
  ] satisfies EChartsOption["graphic"];
}

function buildProjectTimelineOption(summary: NonNullable<ReturnType<typeof useDashboardSummary>["summary"]>, colors: ReturnType<typeof useChartColors>): EChartsOption {
  const buckets = Array.from(new Set(summary.projectTimeline.map((item) => item.bucket)));
  const projects = Array.from(new Set(summary.projectTimeline.map((item) => item.category)));
  const matrix = new Map<string, number>();
  for (const item of summary.projectTimeline) {
    matrix.set(`${item.bucket}__${item.category}`, item.value);
  }

  return {
    color: [colors.accentA, colors.accentB, colors.accentC, colors.accentD, colors.accentE, colors.accentF],
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
    },
    legend: { top: 0, textStyle: { color: colors.axis } },
    grid: { left: 18, right: 18, top: 56, bottom: categoryGridBottom(buckets.length), containLabel: true },
    xAxis: {
      type: "category",
      data: buckets,
      axisLine: { lineStyle: { color: colors.grid } },
      axisLabel: buildCategoryAxisLabel(colors.axis, buckets.length, formatTimelineAxisLabel),
    },
    yAxis: {
      type: "value",
      axisLine: { lineStyle: { color: colors.grid } },
      axisLabel: { color: colors.axis },
      splitLine: { lineStyle: { color: colors.grid } },
    },
    graphic: summary.projectTimeline.length === 0 ? buildEmptyGraphic("当前范围没有项目时间线数据", colors.axis) : undefined,
    series: projects.map((project) => ({
      name: project,
      type: "bar",
      stack: "projects",
      emphasis: { focus: "series" },
      data: buckets.map((bucket) => matrix.get(`${bucket}__${project}`) ?? 0),
    })),
  };
}

function buildParallelOption(summary: NonNullable<ReturnType<typeof useDashboardSummary>["summary"]>, colors: ReturnType<typeof useChartColors>): EChartsOption {
  return {
    color: [colors.accentE],
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
    },
    grid: { left: 18, right: 18, top: 18, bottom: 24, containLabel: true },
    xAxis: {
      type: "value",
      axisLabel: { color: colors.axis },
      axisLine: { lineStyle: { color: colors.grid } },
      splitLine: { lineStyle: { color: colors.grid } },
    },
    yAxis: {
      type: "category",
      data: [...summary.projectParallelism].reverse().map((item) => item.label),
      axisLabel: { color: colors.axis },
      axisLine: { lineStyle: { color: colors.grid } },
    },
    graphic: summary.projectParallelism.length === 0 ? buildEmptyGraphic("当前范围没有并行窗口数据", colors.axis) : undefined,
    series: [
      {
        type: "bar",
        barWidth: 18,
        itemStyle: { borderRadius: [0, 10, 10, 0] },
        data: [...summary.projectParallelism].reverse().map((item) => item.value),
      },
    ],
  };
}

export function ProjectsPage() {
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
  const { summary, loading, error, refresh } = useDashboardSummary(filters, "projects");
  const colors = useChartColors();

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
        eyebrow="Projects"
        title="项目与窗口"
        description="单独观察不同项目的时间线、窗口数量、窗口消耗和并行开展情况。"
        actions={<DashboardHeaderActions baseName="projects" filters={filters} loading={loading} onRefresh={refresh} />}
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

      {loading && !summary ? <Panel title="载入中">正在计算项目与窗口分析…</Panel> : null}
      {error ? <Panel title="载入失败">{error}</Panel> : null}

      {summary ? (
        <>
          <div className="grid gap-6 xl:grid-cols-[2fr,1fr]">
            <Panel title="项目时间线">
              <EChart option={buildProjectTimelineOption(summary, colors)} height={340} />
            </Panel>

            <Panel title="最大并行窗口数">
              <EChart option={buildParallelOption(summary, colors)} height={340} />
            </Panel>
          </div>

          <Panel title="项目概览">
            <div className="overflow-x-auto">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>项目</th>
                    <th>窗口数</th>
                    <th>提问数</th>
                    <th>总 Tokens</th>
                    <th>压缩次数</th>
                    <th>平均首答</th>
                    <th>平均完成</th>
                    <th>最大并行</th>
                  </tr>
                </thead>
                <tbody>
                  {summary.projectSummaries.map((item) => (
                    <tr key={item.label}>
                      <td>{item.label}</td>
                      <td>{formatNumber(item.sessionCount)}</td>
                      <td>{formatNumber(item.questionCount)}</td>
                      <td>{formatNumber(item.totalTokens)}</td>
                      <td>{formatNumber(item.contextCompactions)}</td>
                      <td>{formatDuration(Math.round(item.avgFirstResponseSec))}</td>
                      <td>{formatDuration(Math.round(item.avgCompletionSec))}</td>
                      <td>{formatNumber(item.maxParallelWindows)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Panel>

          <Panel title="窗口消耗排行">
            <div className="overflow-x-auto">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>项目</th>
                    <th>窗口</th>
                    <th>总 Tokens</th>
                    <th>提问数</th>
                    <th>工具</th>
                    <th>时长</th>
                    <th>更新时间</th>
                  </tr>
                </thead>
                <tbody>
                  {summary.projectWindows.map((item) => (
                    <tr key={item.sessionId}>
                      <td>{item.project}</td>
                      <td>
                        <div className="max-w-[28rem]">
                          <div>{formatOptionalText(item.threadTitle, "未命名会话")}</div>
                          <div className="body-text mt-1">{item.sessionId}</div>
                        </div>
                      </td>
                      <td>{formatNumber(item.totalTokens)}</td>
                      <td>{formatNumber(item.questionCount)}</td>
                      <td>{formatNumber(item.toolCallCount)}</td>
                      <td>{formatDuration(item.durationSec)}</td>
                      <td>{formatDateTime(item.updatedAt ?? item.startedAt)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Panel>
        </>
      ) : null}
    </div>
  );
}
