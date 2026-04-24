import { useEffect, useMemo, useState } from "react";
import { getDashboardSummary } from "../../lib/tauri";
import { DashboardFilters, DashboardSection, DashboardSummary } from "../../types/api";

const cache = new Map<string, DashboardSummary>();
const inflight = new Map<string, Promise<DashboardSummary>>();
const allSections: DashboardSection[] = ["overview", "performance", "workflow", "search", "projects", "correlations"];

function normalizeFilters(filters: DashboardFilters) {
  return {
    preset: filters.preset ?? "all",
    granularity: filters.granularity ?? "day",
    dateFrom: filters.dateFrom ?? "",
    dateTo: filters.dateTo ?? "",
    project: filters.project ?? "",
  };
}

function buildCacheKey(section: DashboardSection, filters: DashboardFilters) {
  return `${section}:${JSON.stringify(normalizeFilters(filters))}`;
}

async function loadSummary(section: DashboardSection, filters: DashboardFilters) {
  const key = buildCacheKey(section, filters);
  if (cache.has(key)) {
    return cache.get(key)!;
  }
  if (inflight.has(key)) {
    return inflight.get(key)!;
  }

  const request = getDashboardSummary(filters, section).then((payload) => {
    cache.set(key, payload);
    inflight.delete(key);
    return payload;
  });

  inflight.set(key, request);
  return request;
}

function prefetchSections(currentSection: DashboardSection, filters: DashboardFilters) {
  if (currentSection === "all") {
    return;
  }

  for (const section of allSections) {
    if (section === currentSection) {
      continue;
    }
    const key = buildCacheKey(section, filters);
    if (!cache.has(key) && !inflight.has(key)) {
      void loadSummary(section, filters);
    }
  }
}

export function useDashboardSummary(filters: DashboardFilters, section: DashboardSection) {
  const cacheKey = useMemo(() => buildCacheKey(section, filters), [filters, section]);
  const [summary, setSummary] = useState<DashboardSummary | null>(() => cache.get(cacheKey) ?? null);
  const [loading, setLoading] = useState(!cache.has(cacheKey));
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    const cached = cache.get(cacheKey);
    if (cached) {
      setSummary(cached);
      setLoading(false);
      setError(null);
      prefetchSections(section, filters);
      return () => {
        cancelled = true;
      };
    }

    setLoading(true);
    setError(null);

    void loadSummary(section, filters)
      .then((payload) => {
        if (!cancelled) {
          setSummary(payload);
          setLoading(false);
          prefetchSections(section, filters);
        }
      })
      .catch((cause) => {
        if (!cancelled) {
          setError(cause instanceof Error ? cause.message : "无法加载仪表盘数据。");
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [cacheKey, filters, refreshKey, section]);

  return {
    summary,
    loading,
    error,
    refresh: () => {
      cache.delete(cacheKey);
      inflight.delete(cacheKey);
      setRefreshKey((value) => value + 1);
    },
  };
}
