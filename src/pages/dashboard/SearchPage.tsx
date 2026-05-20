import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { AnalyticsToolbar } from "../../features/dashboard/AnalyticsToolbar";
import { SearchCharts } from "../../features/dashboard/ChartsPanel";
import { DashboardHeaderActions } from "../../features/dashboard/DashboardHeaderActions";
import { useDashboardControls } from "../../features/dashboard/dashboardControls";
import { useDashboardSummary } from "../../features/dashboard/useDashboardSummary";

export function SearchPage() {
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
  const { summary, loading, error, refresh } = useDashboardSummary(filters, "search");

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
        eyebrow="Search"
        title="搜索行为"
        description="单独观察 Web Search 的关键词、使用时段，以及你在什么时间更依赖外部检索。"
        actions={<DashboardHeaderActions baseName="search" filters={filters} loading={loading} onRefresh={refresh} />}
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

      {loading && !summary ? <Panel title="载入中">正在计算搜索行为分析…</Panel> : null}
      {error ? <Panel title="载入失败">{error}</Panel> : null}

      {summary ? <SearchCharts searchKeywords={summary.searchKeywords} searchHours={summary.searchHours} /> : null}
    </div>
  );
}
