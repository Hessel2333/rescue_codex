import type { EChartsOption } from "echarts";
import { useMemo } from "react";
import { useTheme } from "../../app/theme";
import { EChart } from "../../components/EChart";
import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { AnalyticsToolbar } from "../../features/dashboard/AnalyticsToolbar";
import { PerformanceCharts } from "../../features/dashboard/ChartsPanel";
import { DashboardHeaderActions } from "../../features/dashboard/DashboardHeaderActions";
import { useDashboardControls } from "../../features/dashboard/dashboardControls";
import { useDashboardSummary } from "../../features/dashboard/useDashboardSummary";
import { formatDateTime, formatDuration, formatNumber, formatOptionalText } from "../../lib/format";
import { sanitizeMessagePreview } from "../../lib/sessionVisibility";
import { RankedTurnRecord } from "../../types/api";

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
      accentC: read("--chart-accent-c", "#f59e0b"),
      accentD: read("--chart-accent-d", "#f97316"),
    };
  }, [resolvedTheme]);
}

function compactLabel(item: RankedTurnRecord) {
  const text = sanitizeMessagePreview(item.promptPreview) ?? item.threadTitle ?? item.project;
  const singleLine = text.replace(/\s+/g, " ").trim();
  return singleLine.length > 12 ? `${singleLine.slice(0, 12)}…` : singleLine || "未命名";
}

function buildRankingOption(
  items: RankedTurnRecord[],
  colors: ReturnType<typeof useChartColors>,
  mode: "tokens" | "latency",
): EChartsOption {
  const ordered = [...items].reverse();
  const color = mode === "tokens" ? colors.accentC : colors.accentD;

  return {
    color: [color],
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
      formatter: (params: any) => {
        const dataIndex = Array.isArray(params) ? params[0]?.dataIndex ?? 0 : params?.dataIndex ?? 0;
        const item = ordered[dataIndex];
        if (!item) {
          return "";
        }
        return [
          `<strong>${formatOptionalText(item.threadTitle, "未命名会话")}</strong>`,
          `项目: ${item.project}`,
          `消息: ${formatOptionalText(sanitizeMessagePreview(item.promptPreview), "无消息文本")}`,
          mode === "tokens" ? `总 Tokens: ${formatNumber(item.totalTokens)}` : `完成耗时: ${formatDuration(item.completionSec ?? 0)}`,
          `首答耗时: ${formatDuration(item.firstResponseSec ?? 0)}`,
          `输入 / 输出: ${formatNumber(item.inputTokens)} / ${formatNumber(item.outputTokens)}`,
          `缓存输入: ${formatNumber(item.cachedInputTokens)}`,
          `推理输出: ${formatNumber(item.reasoningOutputTokens)}`,
          `时间: ${formatDateTime(item.timestamp)}`,
        ].join("<br/>");
      },
    },
    grid: { left: 20, right: 20, top: 16, bottom: 18, containLabel: true },
    xAxis: {
      type: "value",
      axisLabel: {
        color: colors.axis,
        formatter: (value: number) => (mode === "tokens" ? formatNumber(value) : formatDuration(value)),
      },
      axisLine: { lineStyle: { color: colors.grid } },
      splitLine: { lineStyle: { color: colors.grid } },
    },
    yAxis: {
      type: "category",
      data: ordered.map(compactLabel),
      axisLabel: { color: colors.axis },
      axisLine: { lineStyle: { color: colors.grid } },
    },
    series: [
      {
        type: "bar",
        barWidth: 18,
        itemStyle: { borderRadius: [0, 10, 10, 0] },
        data: ordered.map((item) => (mode === "tokens" ? item.totalTokens : item.completionSec ?? 0)),
      },
    ],
  };
}

export function PerformancePage() {
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
  const { summary, loading, error, refresh } = useDashboardSummary(filters, "performance");
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
        eyebrow="Performance"
        title="等待与模型性能"
        description="聚焦响应速度、整轮完成耗时、工具成功率与平均耗时，以及模型、速度档位和推理强度的变化。"
        actions={<DashboardHeaderActions baseName="performance" filters={filters} loading={loading} onRefresh={refresh} />}
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

      {loading && !summary ? <Panel title="载入中">正在计算性能分析…</Panel> : null}
      {error ? <Panel title="载入失败">{error}</Panel> : null}

      {summary ? (
        <>
          <div className="responsive-metric-grid">
            <Panel title="总 Tokens">
              <div className="metric-card__value">{formatNumber(summary.tokenUsage.totalTokens)}</div>
            </Panel>
            <Panel title="输入">
              <div className="metric-card__value">{formatNumber(summary.tokenUsage.inputTokens)}</div>
            </Panel>
            <Panel title="输出">
              <div className="metric-card__value">{formatNumber(summary.tokenUsage.outputTokens)}</div>
            </Panel>
            <Panel title="缓存输入">
              <div className="metric-card__value">{formatNumber(summary.tokenUsage.cachedInputTokens)}</div>
            </Panel>
            <Panel title="推理输出">
              <div className="metric-card__value">{formatNumber(summary.tokenUsage.reasoningOutputTokens)}</div>
            </Panel>
          </div>

          <PerformanceCharts
            scope={summary.scope}
            durationBuckets={summary.durationBuckets}
            firstTokenBuckets={summary.firstTokenBuckets}
            completionBuckets={summary.completionBuckets}
            toolTypes={summary.toolTypes}
            toolMetrics={summary.toolMetrics}
            modelUsage={summary.modelUsage}
            modelTimeline={summary.modelTimeline}
            reasoningEfforts={summary.reasoningEfforts}
            reasoningTimeline={summary.reasoningTimeline}
            speedTiers={summary.speedTiers}
            speedTimeline={summary.speedTimeline}
          />

          <div className="grid gap-6 xl:grid-cols-2">
            <Panel title="高 Token 消耗消息">
              <EChart option={buildRankingOption(summary.topTokenTurns, colors, "tokens")} height={340} />
            </Panel>

            <Panel title="回复耗时排行">
              <EChart option={buildRankingOption(summary.slowestTurns, colors, "latency")} height={340} />
            </Panel>
          </div>
        </>
      ) : null}
    </div>
  );
}
