import { Download, FileJson2, FileText } from "lucide-react";
import { Button } from "../../components/Button";
import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { AnalyticsToolbar } from "../../features/dashboard/AnalyticsToolbar";
import { WorkflowCharts } from "../../features/dashboard/ChartsPanel";
import { useDashboardControls } from "../../features/dashboard/dashboardControls";
import { useDashboardSummary } from "../../features/dashboard/useDashboardSummary";
import { exportReport, pickSavePath } from "../../lib/tauri";

export function WorkflowPage() {
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
  const { summary, loading, error, refresh } = useDashboardSummary(filters, "workflow");

  async function handleExport(format: "json" | "markdown") {
    const path = await pickSavePath(`rescue_codex-workflow.${format === "markdown" ? "md" : format}`);
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
        eyebrow="Workflow"
        title="提问与工作流"
        description="观察你如何提问、什么时候提问、是否频繁切换项目或工作区，以及交互中断和连接异常的走势。"
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

      {loading && !summary ? <Panel title="载入中">正在计算工作流分析…</Panel> : null}
      {error ? <Panel title="载入失败">{error}</Panel> : null}

      {summary ? (
        <WorkflowCharts
          questionHours={summary.questionHours}
          topPromptTerms={summary.topPromptTerms}
          promptLengthBuckets={summary.promptLengthBuckets}
          promptComposition={summary.promptComposition}
          interruptionTimeline={summary.interruptionTimeline}
          workspaceSwitches={summary.workspaceSwitches}
          workspaceTimeline={summary.workspaceTimeline}
          transportSignals={summary.transportSignals}
          transportTimeline={summary.transportTimeline}
        />
      ) : null}
    </div>
  );
}
