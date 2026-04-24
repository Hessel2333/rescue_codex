import type { EChartsOption } from "echarts";
import { useMemo } from "react";
import { useTheme } from "../../app/theme";
import { EChart } from "../../components/EChart";
import { Panel } from "../../components/Panel";
import { formatDuration, formatNumber } from "../../lib/format";
import { ActivityPoint, BreakdownDatum, ChartDatum, DashboardScope, ToolMetricDatum } from "../../types/api";
import { GitHubHeatmap } from "./GitHubHeatmap";

type OverviewChartsProps = {
  scope: DashboardScope;
  activity: ActivityPoint[];
  heatmapActivity: ActivityPoint[];
};

type PerformanceChartsProps = {
  scope: DashboardScope;
  durationBuckets: ChartDatum[];
  firstTokenBuckets: ChartDatum[];
  completionBuckets: ChartDatum[];
  toolTypes: ChartDatum[];
  toolMetrics: ToolMetricDatum[];
  modelUsage: ChartDatum[];
  modelTimeline: BreakdownDatum[];
  reasoningEfforts: ChartDatum[];
  reasoningTimeline: BreakdownDatum[];
  speedTiers: ChartDatum[];
  speedTimeline: BreakdownDatum[];
};

type WorkflowChartsProps = {
  questionHours: ChartDatum[];
  topPromptTerms: ChartDatum[];
  promptLengthBuckets: ChartDatum[];
  promptComposition: ChartDatum[];
  interruptionTimeline: BreakdownDatum[];
  workspaceSwitches: ChartDatum[];
  workspaceTimeline: BreakdownDatum[];
  transportSignals: ChartDatum[];
  transportTimeline: BreakdownDatum[];
};

type SearchChartsProps = {
  searchKeywords: ChartDatum[];
  searchHours: ChartDatum[];
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
      accentD: read("--chart-accent-d", "#f97316"),
      accentE: read("--chart-accent-e", "#60a5fa"),
      accentF: read("--chart-accent-f", "#f472b6"),
    };
  }, [resolvedTheme]);
}

function buildEmptyGraphic(text: string, axisColor: string) {
  return [
    {
      type: "text",
      left: "center",
      top: "middle",
      style: {
        text,
        fill: axisColor,
        fontFamily: "IBM Plex Sans",
        fontSize: 13,
        align: "center",
      },
    },
  ] satisfies EChartsOption["graphic"];
}

function baseGrid(top = 16) {
  return {
    left: 18,
    right: 18,
    top,
    bottom: 24,
    containLabel: true,
  };
}

function formatActivityAxisLabel(value: string, granularity: DashboardScope["granularity"]) {
  if (granularity === "day" && value.length >= 10) {
    return value.slice(5);
  }
  if (granularity === "week") {
    return value.replace(/^Wk\s+/, "");
  }
  return value;
}

function buildActivityOption(
  activity: ActivityPoint[],
  granularity: DashboardScope["granularity"],
  colors: ReturnType<typeof useChartColors>,
): EChartsOption {
  return {
    color: [colors.accentA, colors.accentB, colors.accentC],
    tooltip: {
      trigger: "axis",
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
      formatter: (params: any) => {
        const items = Array.isArray(params) ? params : [params];
        const lines = [`<div style="margin-bottom:8px">${items[0]?.axisValueLabel ?? ""}</div>`];
        for (const item of items) {
          const numericValue = Number(item.value ?? 0);
          const valueText = item.seriesName.includes("首答") ? formatDuration(numericValue) : formatNumber(numericValue);
          lines.push(`${item.marker}${item.seriesName}: ${valueText}`);
        }
        return lines.join("<br/>");
      },
    },
    legend: {
      top: 0,
      textStyle: { color: colors.axis },
    },
    grid: baseGrid(56),
    xAxis: {
      type: "category",
      boundaryGap: true,
      axisLine: { lineStyle: { color: colors.grid } },
      axisLabel: {
        color: colors.axis,
        formatter: (value: string) => formatActivityAxisLabel(value, granularity),
      },
      data: activity.map((item) => item.date),
    },
    yAxis: [
      {
        type: "value",
        name: "次数",
        nameTextStyle: { color: colors.axis },
        axisLabel: { color: colors.axis },
        axisLine: { lineStyle: { color: colors.grid } },
        splitLine: { lineStyle: { color: colors.grid } },
      },
      {
        type: "value",
        name: "秒",
        nameTextStyle: { color: colors.axis },
        axisLabel: { color: colors.axis, formatter: (value: number) => `${value}s` },
        axisLine: { lineStyle: { color: colors.grid } },
        splitLine: { show: false },
      },
    ],
    graphic: activity.length === 0 ? buildEmptyGraphic("当前范围内没有可展示的趋势数据", colors.axis) : undefined,
    series: [
      {
        name: "会话数",
        type: "bar",
        barMaxWidth: 18,
        itemStyle: { borderRadius: [8, 8, 0, 0] },
        data: activity.map((item) => item.sessions),
      },
      {
        name: "提问数",
        type: "line",
        smooth: true,
        symbolSize: 7,
        data: activity.map((item) => item.questions),
      },
      {
        name: "平均首答耗时",
        type: "line",
        smooth: true,
        yAxisIndex: 1,
        symbolSize: 6,
        lineStyle: { width: 2, type: "dashed" },
        data: activity.map((item) => item.avgFirstResponseSec),
      },
    ],
  };
}

function buildVerticalBarOption(
  data: ChartDatum[],
  color: string,
  colors: ReturnType<typeof useChartColors>,
  valueFormatter?: (value: number) => string,
  maxValue?: number,
): EChartsOption {
  return {
    color: [color],
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
      formatter: (params: any) => {
        const item = Array.isArray(params) ? params[0] : params;
        const value = Number(item?.value ?? 0);
        return `${item?.axisValueLabel ?? ""}<br/>${item?.marker ?? ""}${valueFormatter ? valueFormatter(value) : formatNumber(value)}`;
      },
    },
    grid: baseGrid(),
    xAxis: {
      type: "category",
      axisLabel: { color: colors.axis, interval: 0 },
      axisLine: { lineStyle: { color: colors.grid } },
      data: data.map((item) => item.label),
    },
    yAxis: {
      type: "value",
      max: maxValue,
      axisLabel: { color: colors.axis },
      axisLine: { lineStyle: { color: colors.grid } },
      splitLine: { lineStyle: { color: colors.grid } },
    },
    graphic: data.length === 0 ? buildEmptyGraphic("当前范围内没有统计结果", colors.axis) : undefined,
    series: [
      {
        type: "bar",
        barMaxWidth: 26,
        itemStyle: { borderRadius: [10, 10, 0, 0] },
        data: data.map((item) => item.value),
      },
    ],
  };
}

function buildHorizontalBarOption(
  data: ChartDatum[],
  color: string,
  colors: ReturnType<typeof useChartColors>,
  valueFormatter?: (value: number) => string,
): EChartsOption {
  const reversed = [...data].reverse();
  return {
    color: [color],
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
      formatter: (params: any) => {
        const item = Array.isArray(params) ? params[0] : params;
        const value = Number(item?.value ?? 0);
        return `${item?.name ?? ""}<br/>${item?.marker ?? ""}${valueFormatter ? valueFormatter(value) : formatNumber(value)}`;
      },
    },
    grid: {
      left: 16,
      right: 24,
      top: 10,
      bottom: 16,
      containLabel: true,
    },
    xAxis: {
      type: "value",
      axisLabel: { color: colors.axis },
      axisLine: { lineStyle: { color: colors.grid } },
      splitLine: { lineStyle: { color: colors.grid } },
    },
    yAxis: {
      type: "category",
      axisLine: { lineStyle: { color: colors.grid } },
      axisLabel: {
        color: colors.axis,
        formatter: (value: string) => (value.length > 20 ? `${value.slice(0, 20)}...` : value),
      },
      data: reversed.map((item) => item.label),
    },
    graphic: data.length === 0 ? buildEmptyGraphic("当前范围内没有统计结果", colors.axis) : undefined,
    series: [
      {
        type: "bar",
        barWidth: 18,
        itemStyle: { borderRadius: [0, 10, 10, 0] },
        data: reversed.map((item) => item.value),
      },
    ],
  };
}

function buildPieOption(data: ChartDatum[], title: string, colors: ReturnType<typeof useChartColors>): EChartsOption {
  return {
    color: [colors.accentA, colors.accentB, colors.accentC, colors.accentD, colors.accentE, colors.accentF],
    tooltip: {
      trigger: "item",
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
      formatter: (params: any) =>
        `${params.name}<br/>${params.marker}${formatNumber(Number(params.value ?? 0))} (${params.percent ?? 0}%)`,
    },
    legend: {
      bottom: 0,
      icon: "circle",
      textStyle: { color: colors.axis },
    },
    graphic:
      data.length === 0
        ? buildEmptyGraphic("当前范围内没有统计结果", colors.axis)
        : [
            {
              type: "text",
              left: "center",
              top: "38%",
              style: {
                text: title,
                fill: colors.tooltipText,
                fontFamily: "IBM Plex Sans",
                fontSize: 13,
                fontWeight: 600,
                align: "center",
              },
            },
          ],
    series: [
      {
        type: "pie",
        radius: ["48%", "72%"],
        center: ["50%", "42%"],
        avoidLabelOverlap: true,
        label: { color: colors.axis, formatter: "{b}\n{d}%" },
        labelLine: { lineStyle: { color: colors.grid } },
        data: data.map((item) => ({ name: item.label, value: item.value })),
      },
    ],
  };
}

function buildBreakdownOption(
  data: BreakdownDatum[],
  colors: ReturnType<typeof useChartColors>,
  palette: string[],
): EChartsOption {
  const buckets = Array.from(new Set(data.map((item) => item.bucket)));
  const categories = Array.from(new Set(data.map((item) => item.category)));
  const matrix = new Map<string, number>();
  for (const item of data) {
    matrix.set(`${item.bucket}__${item.category}`, item.value);
  }

  return {
    color: palette,
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
    },
    legend: {
      top: 0,
      textStyle: { color: colors.axis },
    },
    grid: baseGrid(52),
    xAxis: {
      type: "category",
      axisLabel: { color: colors.axis, interval: 0 },
      axisLine: { lineStyle: { color: colors.grid } },
      data: buckets,
    },
    yAxis: {
      type: "value",
      axisLabel: { color: colors.axis },
      axisLine: { lineStyle: { color: colors.grid } },
      splitLine: { lineStyle: { color: colors.grid } },
    },
    graphic: data.length === 0 ? buildEmptyGraphic("当前范围内没有时间线数据", colors.axis) : undefined,
    series: categories.map((category) => ({
      name: category,
      type: "bar",
      stack: "total",
      emphasis: { focus: "series" },
      data: buckets.map((bucket) => matrix.get(`${bucket}__${category}`) ?? 0),
    })),
  };
}

function buildToolOutcomeOption(toolMetrics: ToolMetricDatum[], colors: ReturnType<typeof useChartColors>): EChartsOption {
  const ordered = [...toolMetrics].reverse();
  return {
    color: [colors.accentA, colors.accentD],
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      backgroundColor: colors.tooltipBg,
      borderColor: colors.grid,
      textStyle: { color: colors.tooltipText },
      formatter: (params: any) => {
        const items = Array.isArray(params) ? params : [params];
        const label = items[0]?.name ?? "";
        const metric = toolMetrics.find((item) => item.label === label);
        return [
          label,
          ...items.map((item) => `${item.marker}${item.seriesName}: ${formatNumber(Number(item.value ?? 0))}`),
          metric ? `平均耗时: ${formatDuration(Math.round(metric.avgDurationSec))}` : "",
        ]
          .filter(Boolean)
          .join("<br/>");
      },
    },
    legend: {
      top: 0,
      textStyle: { color: colors.axis },
    },
    grid: { left: 20, right: 24, top: 52, bottom: 16, containLabel: true },
    xAxis: {
      type: "value",
      axisLabel: { color: colors.axis },
      axisLine: { lineStyle: { color: colors.grid } },
      splitLine: { lineStyle: { color: colors.grid } },
    },
    yAxis: {
      type: "category",
      axisLabel: { color: colors.axis },
      axisLine: { lineStyle: { color: colors.grid } },
      data: ordered.map((item) => item.label),
    },
    graphic: toolMetrics.length === 0 ? buildEmptyGraphic("当前范围内没有工具结果数据", colors.axis) : undefined,
    series: [
      {
        name: "成功",
        type: "bar",
        stack: "tool",
        itemStyle: { borderRadius: [0, 10, 10, 0] },
        data: ordered.map((item) => item.success),
      },
      {
        name: "失败",
        type: "bar",
        stack: "tool",
        data: ordered.map((item) => item.failure),
      },
    ],
  };
}

function buildToolDurationOption(toolMetrics: ToolMetricDatum[], colors: ReturnType<typeof useChartColors>): EChartsOption {
  const data = toolMetrics.map((item) => ({
    label: item.label,
    value: Math.round(item.avgDurationSec),
  }));
  return buildHorizontalBarOption(data, colors.accentE, colors, (value) => formatDuration(value));
}

export function OverviewCharts({ scope, activity, heatmapActivity }: OverviewChartsProps) {
  const colors = useChartColors();

  return (
    <div className="grid gap-6">
      <Panel title="活跃趋势">
        <EChart option={buildActivityOption(activity, scope.granularity, colors)} height={360} />
      </Panel>

      <Panel title="活跃热力图">
        <GitHubHeatmap data={heatmapActivity} />
      </Panel>
    </div>
  );
}

export function PerformanceCharts({
  durationBuckets,
  firstTokenBuckets,
  completionBuckets,
  toolTypes,
  toolMetrics,
  modelUsage,
  modelTimeline,
  reasoningEfforts,
  reasoningTimeline,
  speedTiers,
  speedTimeline,
}: PerformanceChartsProps) {
  const colors = useChartColors();

  return (
    <div className="grid gap-6 2xl:grid-cols-2">
      <Panel title="首 Token 耗时分布">
        <EChart option={buildVerticalBarOption(firstTokenBuckets, colors.accentC, colors, (value) => formatDuration(value))} height={300} />
      </Panel>

      <Panel title="整轮完成耗时分布">
        <EChart option={buildVerticalBarOption(completionBuckets, colors.accentB, colors, (value) => formatDuration(value))} height={300} />
      </Panel>

      <Panel title="会话时长分布">
        <EChart option={buildVerticalBarOption(durationBuckets, colors.accentA, colors)} height={300} />
      </Panel>

      <Panel title="工具调用类型">
        <EChart option={buildHorizontalBarOption(toolTypes, colors.accentA, colors)} height={320} />
      </Panel>

      <Panel title="工具成功 / 失败">
        <EChart option={buildToolOutcomeOption(toolMetrics, colors)} height={340} />
      </Panel>

      <Panel title="工具平均耗时">
        <EChart option={buildToolDurationOption(toolMetrics, colors)} height={340} />
      </Panel>

      <Panel title="模型分布">
        <EChart option={buildPieOption(modelUsage, "模型", colors)} height={320} />
      </Panel>

      <Panel title="模型时间线">
        <EChart option={buildBreakdownOption(modelTimeline, colors, [colors.accentA, colors.accentB, colors.accentC, colors.accentD, colors.accentE])} height={320} />
      </Panel>

      <Panel title="推理强度分布">
        <EChart option={buildPieOption(reasoningEfforts, "推理强度", colors)} height={320} />
      </Panel>

      <Panel title="推理强度时间线">
        <EChart option={buildBreakdownOption(reasoningTimeline, colors, [colors.accentA, colors.accentC, colors.accentD, colors.accentE])} height={320} />
      </Panel>

      <Panel title="速度档位分布">
        <EChart option={buildPieOption(speedTiers, "速度档位", colors)} height={320} />
      </Panel>

      <Panel title="速度档位时间线">
        <EChart option={buildBreakdownOption(speedTimeline, colors, [colors.accentA, colors.accentB, colors.accentC, colors.axis])} height={320} />
      </Panel>
    </div>
  );
}

export function WorkflowCharts({
  questionHours,
  topPromptTerms,
  promptLengthBuckets,
  promptComposition,
  interruptionTimeline,
  workspaceSwitches,
  workspaceTimeline,
  transportSignals,
  transportTimeline,
}: WorkflowChartsProps) {
  const colors = useChartColors();

  return (
    <div className="grid gap-6 2xl:grid-cols-2">
      <Panel title="提问时间分布">
        <EChart option={buildVerticalBarOption(questionHours, colors.accentA, colors)} height={300} />
      </Panel>

      <Panel title="提问长度分布">
        <EChart option={buildVerticalBarOption(promptLengthBuckets, colors.accentD, colors)} height={300} />
      </Panel>

      <Panel title="提问内容构成">
        <EChart option={buildVerticalBarOption(promptComposition, colors.accentE, colors, (value) => `${value}%`, 100)} height={300} />
      </Panel>

      <Panel title="高频提问词 / 字段">
        <EChart option={buildHorizontalBarOption(topPromptTerms, colors.accentD, colors)} height={320} />
      </Panel>

      <Panel title="连接诊断信号">
        <EChart option={buildVerticalBarOption(transportSignals, colors.accentE, colors)} height={300} />
      </Panel>

      <Panel title="连接诊断时间线">
        <EChart option={buildBreakdownOption(transportTimeline, colors, [colors.accentE, colors.accentC, colors.accentD])} height={320} />
      </Panel>

      <Panel title="中断 / 回滚 / 压缩">
        <EChart option={buildBreakdownOption(interruptionTimeline, colors, [colors.accentD, colors.accentE, colors.accentA])} height={320} />
      </Panel>

      <Panel title="项目切换与工作区切换">
        <EChart option={buildVerticalBarOption(workspaceSwitches, colors.accentF, colors)} height={300} />
      </Panel>

      <Panel title="工作区切换时间线" className="2xl:col-span-2">
        <EChart option={buildBreakdownOption(workspaceTimeline, colors, [colors.accentF, colors.accentE])} height={320} />
      </Panel>
    </div>
  );
}

export function SearchCharts({ searchKeywords, searchHours }: SearchChartsProps) {
  const colors = useChartColors();

  return (
    <div className="grid gap-6 2xl:grid-cols-2">
      <Panel title="搜索时间分布">
        <EChart option={buildVerticalBarOption(searchHours, colors.accentC, colors)} height={300} />
      </Panel>

      <Panel title="搜索关键词">
        <EChart option={buildHorizontalBarOption(searchKeywords, colors.accentB, colors)} height={320} />
      </Panel>
    </div>
  );
}
