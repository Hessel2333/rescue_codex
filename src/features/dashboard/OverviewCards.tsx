import { Activity, Clock3, Gauge, MessageSquareQuote, Sparkles, Wrench } from "lucide-react";
import { StatCard } from "../../components/StatCard";
import { formatDuration, formatNumber } from "../../lib/format";
import { DashboardOverview, DashboardScope } from "../../types/api";

type OverviewCardsProps = {
  overview: DashboardOverview;
  scope: DashboardScope;
};

function granularityLabel(granularity: DashboardScope["granularity"]) {
  switch (granularity) {
    case "week":
      return "按周";
    case "month":
      return "按月";
    case "year":
      return "按年";
    default:
      return "按天";
  }
}

export function OverviewCards({ overview, scope }: OverviewCardsProps) {
  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-6">
      <StatCard
        label="总会话数"
        value={formatNumber(overview.totalSessions)}
        hint={`统计区间 ${scope.dateFrom} 至 ${scope.dateTo}`}
        icon={<Activity className="h-5 w-5 text-emerald-300" />}
      />
      <StatCard
        label="总提问数"
        value={formatNumber(overview.totalQuestions)}
        hint={`活跃趋势当前按 ${granularityLabel(scope.granularity)} 聚合`}
        icon={<MessageSquareQuote className="h-5 w-5 text-cyan-300" />}
      />
      <StatCard
        label="活跃天数"
        value={formatNumber(overview.activeDays)}
        hint={`近 7 天 ${formatNumber(overview.sessionsLast7Days)}，近 30 天 ${formatNumber(overview.sessionsLast30Days)}`}
        icon={<Clock3 className="h-5 w-5 text-amber-300" />}
      />
      <StatCard
        label="平均首 token 耗时"
        value={formatDuration(Math.round(overview.avgFirstResponseSec))}
        hint="从你发出问题到出现首个助手可见响应的平均耗时"
        icon={<Gauge className="h-5 w-5 text-emerald-300" />}
      />
      <StatCard
        label="平均整轮耗时"
        value={formatDuration(Math.round(overview.avgTurnCompletionSec))}
        hint={`平均 ${overview.avgTurnCount.toFixed(1)} turns，更接近完整处理耗时`}
        icon={<Sparkles className="h-5 w-5 text-cyan-300" />}
      />
      <StatCard
        label="工具调用数"
        value={formatNumber(overview.totalToolCalls)}
        hint={`当前区间共发生 ${formatNumber(overview.totalToolCalls)} 次工具调用`}
        icon={<Wrench className="h-5 w-5 text-amber-300" />}
      />
    </div>
  );
}
