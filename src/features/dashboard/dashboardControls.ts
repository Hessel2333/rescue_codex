import { useEffect, useMemo, useState } from "react";
import { DashboardFilters, DashboardGranularity } from "../../types/api";

export type DashboardTimeUnit = "week" | "month" | "year" | "all";

const unitKey = "rescue_codex.analytics.unit";
const anchorKey = "rescue_codex.analytics.anchor";
const granularityKey = "rescue_codex.analytics.granularity";
const projectKey = "rescue_codex.analytics.project";

function parseDate(value: string) {
  const [year, month, day] = value.split("-").map(Number);
  return new Date(year, month - 1, day, 12, 0, 0, 0);
}

function formatDate(value: Date) {
  const year = value.getFullYear();
  const month = `${value.getMonth() + 1}`.padStart(2, "0");
  const day = `${value.getDate()}`.padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function addDays(value: Date, days: number) {
  const next = new Date(value);
  next.setDate(next.getDate() + days);
  return next;
}

function addMonths(value: Date, months: number) {
  const next = new Date(value);
  next.setMonth(next.getMonth() + months, 1);
  return next;
}

function addYears(value: Date, years: number) {
  const next = new Date(value);
  next.setFullYear(next.getFullYear() + years, 0, 1);
  return next;
}

function startOfWeek(value: Date) {
  const next = new Date(value);
  const day = next.getDay();
  const delta = day === 0 ? -6 : 1 - day;
  next.setDate(next.getDate() + delta);
  return next;
}

function endOfWeek(value: Date) {
  return addDays(startOfWeek(value), 6);
}

function startOfMonth(value: Date) {
  return new Date(value.getFullYear(), value.getMonth(), 1, 12, 0, 0, 0);
}

function endOfMonth(value: Date) {
  return new Date(value.getFullYear(), value.getMonth() + 1, 0, 12, 0, 0, 0);
}

function startOfYear(value: Date) {
  return new Date(value.getFullYear(), 0, 1, 12, 0, 0, 0);
}

function endOfYear(value: Date) {
  return new Date(value.getFullYear(), 11, 31, 12, 0, 0, 0);
}

function getIsoWeek(value: Date) {
  const date = new Date(value);
  const day = (date.getDay() + 6) % 7;
  date.setDate(date.getDate() - day + 3);
  const firstThursday = new Date(date.getFullYear(), 0, 4);
  const firstDay = (firstThursday.getDay() + 6) % 7;
  firstThursday.setDate(firstThursday.getDate() - firstDay + 3);
  return 1 + Math.round((date.getTime() - firstThursday.getTime()) / 604800000);
}

export function periodLabel(timeUnit: DashboardTimeUnit, anchorDate: string) {
  if (timeUnit === "all") {
    return "全部时间";
  }
  const anchor = parseDate(anchorDate);
  if (timeUnit === "week") {
    return `${anchor.getFullYear()} 年第 ${getIsoWeek(anchor)} 周`;
  }
  if (timeUnit === "month") {
    return `${anchor.getFullYear()} 年 ${anchor.getMonth() + 1} 月`;
  }
  return `${anchor.getFullYear()} 年`;
}

export function shiftAnchor(anchorDate: string, timeUnit: DashboardTimeUnit, direction: number) {
  const anchor = parseDate(anchorDate);
  if (timeUnit === "week") {
    return formatDate(addDays(anchor, direction * 7));
  }
  if (timeUnit === "month") {
    return formatDate(addMonths(anchor, direction));
  }
  if (timeUnit === "year") {
    return formatDate(addYears(anchor, direction));
  }
  return anchorDate;
}

export function buildFilters(
  timeUnit: DashboardTimeUnit,
  anchorDate: string,
  granularity: DashboardGranularity,
  project: string,
): DashboardFilters {
  if (timeUnit === "all") {
    return {
      preset: "all",
      granularity,
      project: project === "all" ? undefined : project,
    };
  }

  const anchor = parseDate(anchorDate);
  let start = anchor;
  let end = anchor;

  if (timeUnit === "week") {
    start = startOfWeek(anchor);
    end = endOfWeek(anchor);
  } else if (timeUnit === "month") {
    start = startOfMonth(anchor);
    end = endOfMonth(anchor);
  } else {
    start = startOfYear(anchor);
    end = endOfYear(anchor);
  }

  return {
    preset: "custom",
    granularity,
    dateFrom: formatDate(start),
    dateTo: formatDate(end),
    project: project === "all" ? undefined : project,
  };
}

export function useDashboardControls() {
  const today = formatDate(new Date());
  const [timeUnit, setTimeUnit] = useState<DashboardTimeUnit>(() => {
    if (typeof window === "undefined") {
      return "month";
    }
    const saved = window.localStorage.getItem(unitKey);
    return saved === "week" || saved === "month" || saved === "year" || saved === "all" ? saved : "month";
  });
  const [anchorDate, setAnchorDate] = useState(() => {
    if (typeof window === "undefined") {
      return today;
    }
    return window.localStorage.getItem(anchorKey) ?? today;
  });
  const [granularity, setGranularity] = useState<DashboardGranularity>(() => {
    if (typeof window === "undefined") {
      return "day";
    }
    const saved = window.localStorage.getItem(granularityKey);
    return saved === "day" || saved === "week" || saved === "month" || saved === "year" ? saved : "day";
  });
  const [project, setProject] = useState(() => {
    if (typeof window === "undefined") {
      return "all";
    }
    return window.localStorage.getItem(projectKey) ?? "all";
  });

  useEffect(() => {
    window.localStorage.setItem(unitKey, timeUnit);
  }, [timeUnit]);

  useEffect(() => {
    window.localStorage.setItem(anchorKey, anchorDate);
  }, [anchorDate]);

  useEffect(() => {
    window.localStorage.setItem(granularityKey, granularity);
  }, [granularity]);

  useEffect(() => {
    window.localStorage.setItem(projectKey, project);
  }, [project]);

  const filters = useMemo(
    () => buildFilters(timeUnit, anchorDate, granularity, project),
    [anchorDate, granularity, project, timeUnit],
  );

  return {
    timeUnit,
    setTimeUnit,
    anchorDate,
    setAnchorDate,
    granularity,
    setGranularity,
    project,
    setProject,
    filters,
  };
}
