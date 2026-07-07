import { ChevronLeft, ChevronRight } from "lucide-react";
import { DashboardGranularity } from "../../types/api";
import { DashboardTimeUnit, periodLabel, shiftAnchor } from "./dashboardControls";

const timeUnitOptions: Array<{ value: DashboardTimeUnit; label: string }> = [
  { value: "week", label: "周" },
  { value: "month", label: "月份" },
  { value: "year", label: "年份" },
  { value: "all", label: "全部" },
];

const granularityOptions: Array<{ value: DashboardGranularity; label: string }> = [
  { value: "day", label: "按天" },
  { value: "week", label: "按周" },
  { value: "month", label: "按月" },
  { value: "year", label: "按年" },
];

function ToolbarButton({
  active,
  children,
  onClick,
}: {
  active: boolean;
  children: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={active ? "toolbar-chip is-active" : "toolbar-chip"}
      aria-pressed={active}
    >
      {children}
    </button>
  );
}

type AnalyticsToolbarProps = {
  timeUnit: DashboardTimeUnit;
  setTimeUnit: (value: DashboardTimeUnit) => void;
  anchorDate: string;
  setAnchorDate: (value: string | ((current: string) => string)) => void;
  granularity: DashboardGranularity;
  setGranularity: (value: DashboardGranularity) => void;
  project: string;
  setProject: (value: string) => void;
  projectOptions: string[];
  previousDisabled: boolean;
  nextDisabled: boolean;
};

export function AnalyticsToolbar({
  timeUnit,
  setTimeUnit,
  anchorDate,
  setAnchorDate,
  granularity,
  setGranularity,
  project,
  setProject,
  projectOptions,
  previousDisabled,
  nextDisabled,
}: AnalyticsToolbarProps) {
  return (
    <div className="dashboard-toolbar sticky top-4 z-20">
      <div className="dashboard-toolbar__inner">
        <div className="dashboard-toolbar__group">
          {timeUnitOptions.map((option) => (
            <ToolbarButton key={option.value} active={timeUnit === option.value} onClick={() => setTimeUnit(option.value)}>
              {option.label}
            </ToolbarButton>
          ))}
        </div>

        <div className="dashboard-toolbar__navigator">
          <button
            type="button"
            className="dashboard-toolbar__icon"
            onClick={() => setAnchorDate((current) => shiftAnchor(current, timeUnit, -1))}
            disabled={previousDisabled}
            aria-label="上一时间段"
          >
            <ChevronLeft className="h-4 w-4" />
          </button>

          <div className="dashboard-toolbar__period">
            <div className="dashboard-toolbar__period-label">{periodLabel(timeUnit, anchorDate)}</div>
          </div>

          <button
            type="button"
            className="dashboard-toolbar__icon"
            onClick={() => setAnchorDate((current) => shiftAnchor(current, timeUnit, 1))}
            disabled={nextDisabled}
            aria-label="下一时间段"
          >
            <ChevronRight className="h-4 w-4" />
          </button>
        </div>

        <div className="dashboard-toolbar__group">
          <select className="toolbar-select" value={project} onChange={(event) => setProject(event.target.value)}>
            <option value="all">全部项目</option>
            {projectOptions.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        </div>

        <div className="dashboard-toolbar__group">
          {granularityOptions.map((option) => (
            <ToolbarButton
              key={option.value}
              active={granularity === option.value}
              onClick={() => setGranularity(option.value)}
            >
              {option.label}
            </ToolbarButton>
          ))}
        </div>
      </div>
    </div>
  );
}
