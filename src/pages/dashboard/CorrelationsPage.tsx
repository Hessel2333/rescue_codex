import type { EChartsOption } from "echarts";
import { useMemo } from "react";
import { useTheme } from "../../app/theme";
import { chartFontFamily, EChart } from "../../components/EChart";
import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { AnalyticsToolbar } from "../../features/dashboard/AnalyticsToolbar";
import { DashboardHeaderActions } from "../../features/dashboard/DashboardHeaderActions";
import { useDashboardControls } from "../../features/dashboard/dashboardControls";
import { useDashboardSummary } from "../../features/dashboard/useDashboardSummary";
import { formatDuration, formatNumber } from "../../lib/format";
import { ScatterDatum } from "../../types/api";

type YMetric = "completionSec" | "totalTokens";

type ScatterChartConfig = {
  title: string;
  data: ScatterDatum[];
  xLabel: string;
  yMetric: YMetric;
  xFormatter?: (value: number) => string;
};

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

function formatYValue(value: number, metric: YMetric) {
  return metric === "completionSec" ? formatDuration(Math.round(value)) : formatNumber(Math.round(value));
}

function buildRegressionLine(data: ScatterDatum[], metric: YMetric) {
  if (data.length < 2) {
    return [];
  }

  const points = data.map((item) => ({ x: item.x, y: metric === "completionSec" ? item.completionSec : item.totalTokens }));
  const n = points.length;
  const sumX = points.reduce((total, item) => total + item.x, 0);
  const sumY = points.reduce((total, item) => total + item.y, 0);
  const sumXY = points.reduce((total, item) => total + item.x * item.y, 0);
  const sumXX = points.reduce((total, item) => total + item.x * item.x, 0);
  const denominator = n * sumXX - sumX * sumX;
  if (denominator === 0) {
    return [];
  }

  const slope = (n * sumXY - sumX * sumY) / denominator;
  const intercept = (sumY - slope * sumX) / n;
  const minX = Math.min(...points.map((item) => item.x));
  const maxX = Math.max(...points.map((item) => item.x));
  return [
    [minX, slope * minX + intercept],
    [maxX, slope * maxX + intercept],
  ];
}

function buildScatterOption(
  config: ScatterChartConfig,
  colors: ReturnType<typeof useChartColors>,
): EChartsOption {
  const points = config.data.map((item) => [item.x, config.yMetric === "completionSec" ? item.completionSec : item.totalTokens]);
  const regression = buildRegressionLine(config.data, config.yMetric);

  return {
    color: [colors.accentA, colors.accentC],
    tooltip: {
      trigger: "item",
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
      formatter: (params: any) => {
        const dataIndex = typeof params?.dataIndex === "number" ? params.dataIndex : -1;
        const sample = dataIndex >= 0 ? config.data[dataIndex] : null;
        const raw = Array.isArray(params?.data) ? params.data : null;
        if (!sample || !raw) {
          return "";
        }

        const xValue = Number(raw[0] ?? 0);
        const yValue = Number(raw[1] ?? 0);
        return [
          `<strong>${sample.label}</strong>`,
          `${config.xLabel}: ${config.xFormatter ? config.xFormatter(xValue) : formatNumber(Math.round(xValue))}`,
          `${config.yMetric === "completionSec" ? "完成耗时" : "总 Tokens"}: ${formatYValue(yValue, config.yMetric)}`,
          `首答耗时: ${formatDuration(Math.round(sample.firstResponseSec ?? 0))}`,
          `总 Tokens: ${formatNumber(Math.round(sample.totalTokens ?? 0))}`,
          `Token / 秒: ${Number(sample.tokenRate ?? 0).toFixed(2)}`,
          sample.detail ? `<div style="max-width:320px; white-space:normal; margin-top:6px;">${sample.detail}</div>` : "",
        ].join("<br/>");
      },
    },
    grid: {
      left: 18,
      right: 18,
      top: 18,
      bottom: 24,
      containLabel: true,
    },
    xAxis: {
      type: "value",
      name: config.xLabel,
      nameTextStyle: { color: colors.axis },
      axisLabel: {
        color: colors.axis,
        formatter: (value: number) => (config.xFormatter ? config.xFormatter(value) : `${Math.round(value)}`),
      },
      axisLine: { lineStyle: { color: colors.grid } },
      splitLine: { lineStyle: { color: colors.grid } },
    },
    yAxis: {
      type: "value",
      name: config.yMetric === "completionSec" ? "完成耗时" : "总 Tokens",
      nameTextStyle: { color: colors.axis },
      axisLabel: {
        color: colors.axis,
        formatter: (value: number) => formatYValue(value, config.yMetric),
      },
      axisLine: { lineStyle: { color: colors.grid } },
      splitLine: { lineStyle: { color: colors.grid } },
    },
    graphic: config.data.length === 0 ? buildEmptyGraphic("当前范围内没有可分析的相关性样本", colors.axis) : undefined,
    series: [
      {
        name: "样本点",
        type: "scatter",
        data: points,
        symbolSize: 9,
        itemStyle: {
          color: colors.accentA,
          opacity: 0.56,
        },
        emphasis: {
          itemStyle: {
            color: colors.accentB,
            opacity: 0.9,
          },
        },
      },
      regression.length > 1
        ? {
            name: "趋势线",
            type: "line",
            data: regression,
            showSymbol: false,
            smooth: false,
            tooltip: { show: false },
            lineStyle: {
              width: 2,
              type: "dashed",
              color: colors.accentC,
            },
          }
        : {
            name: "趋势线",
            type: "line",
            data: [],
          },
    ],
  };
}

export function CorrelationsPage() {
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
  const { summary, loading, error, refresh } = useDashboardSummary(filters, "correlations");
  const colors = useChartColors();

  const availableFrom = summary?.scope.availableFrom;
  const availableTo = summary?.scope.availableTo;
  const currentScopeText = summary ? `${summary.scope.dateFrom} 至 ${summary.scope.dateTo}` : "载入中";
  const previousDisabled =
    timeUnit === "all" || !availableFrom || (filters.dateFrom ? filters.dateFrom <= availableFrom : false);
  const nextDisabled =
    timeUnit === "all" || !availableTo || (filters.dateTo ? filters.dateTo >= availableTo : false);

  const charts: ScatterChartConfig[] = summary
    ? [
        {
          title: "提问时段 vs 完成耗时",
          data: summary.hourlyCorrelationScatter,
          xLabel: "提问时段",
          yMetric: "completionSec",
          xFormatter: (value) => `${Math.round(value).toString().padStart(2, "0")}:00`,
        },
        {
          title: "提问时段 vs 总 Tokens",
          data: summary.hourlyCorrelationScatter,
          xLabel: "提问时段",
          yMetric: "totalTokens",
          xFormatter: (value) => `${Math.round(value).toString().padStart(2, "0")}:00`,
        },
        {
          title: "工作日 / 周末 vs 完成耗时",
          data: summary.weekdayCorrelationScatter,
          xLabel: "日期类型",
          yMetric: "completionSec",
          xFormatter: (value) => (value >= 0.5 ? "周末" : "工作日"),
        },
        {
          title: "工作日 / 周末 vs 总 Tokens",
          data: summary.weekdayCorrelationScatter,
          xLabel: "日期类型",
          yMetric: "totalTokens",
          xFormatter: (value) => (value >= 0.5 ? "周末" : "工作日"),
        },
        {
          title: "消息长度 vs 完成耗时",
          data: summary.promptLengthCorrelationScatter,
          xLabel: "消息长度",
          yMetric: "completionSec",
          xFormatter: (value) => `${Math.round(value)} 字`,
        },
        {
          title: "消息长度 vs 总 Tokens",
          data: summary.promptLengthCorrelationScatter,
          xLabel: "消息长度",
          yMetric: "totalTokens",
          xFormatter: (value) => `${Math.round(value)} 字`,
        },
        {
          title: "工具调用次数 vs 完成耗时",
          data: summary.toolLoadCorrelationScatter,
          xLabel: "工具调用次数",
          yMetric: "completionSec",
          xFormatter: (value) => `${Math.round(value)} 次`,
        },
        {
          title: "工具调用次数 vs 总 Tokens",
          data: summary.toolLoadCorrelationScatter,
          xLabel: "工具调用次数",
          yMetric: "totalTokens",
          xFormatter: (value) => `${Math.round(value)} 次`,
        },
        {
          title: "上下文次数 vs 完成耗时",
          data: summary.contextLoadCorrelationScatter,
          xLabel: "上下文次数",
          yMetric: "completionSec",
          xFormatter: (value) => `${Math.round(value)} 次`,
        },
        {
          title: "上下文次数 vs 总 Tokens",
          data: summary.contextLoadCorrelationScatter,
          xLabel: "上下文次数",
          yMetric: "totalTokens",
          xFormatter: (value) => `${Math.round(value)} 次`,
        },
      ]
    : [];

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title="相关性分析"
        description="每张图只保留一个 X 与一个 Y 的关系，专门用来观察线性趋势和离群点。"
        actions={<DashboardHeaderActions baseName="correlations" filters={filters} loading={loading} onRefresh={refresh} />}
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

      {loading && !summary ? <Panel title="载入中">正在计算相关性分析…</Panel> : null}
      {error ? <Panel title="载入失败">{error}</Panel> : null}

      {summary ? (
        <div className="grid gap-6 xl:grid-cols-2">
          {charts.map((chart) => (
            <Panel key={chart.title} title={chart.title}>
              <EChart option={buildScatterOption(chart, colors)} height={320} />
            </Panel>
          ))}
        </div>
      ) : null}
    </div>
  );
}
