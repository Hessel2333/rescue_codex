import { lazy, ReactNode, Suspense } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./layout/AppShell";
import { ThemeProvider } from "./theme";

const DashboardPage = lazy(() =>
  import("../pages/dashboard/DashboardPage").then((module) => ({ default: module.DashboardPage })),
);
const CorrelationsPage = lazy(() =>
  import("../pages/dashboard/CorrelationsPage").then((module) => ({ default: module.CorrelationsPage })),
);
const PerformancePage = lazy(() =>
  import("../pages/dashboard/PerformancePage").then((module) => ({ default: module.PerformancePage })),
);
const WorkflowPage = lazy(() =>
  import("../pages/dashboard/WorkflowPage").then((module) => ({ default: module.WorkflowPage })),
);
const SearchPage = lazy(() =>
  import("../pages/dashboard/SearchPage").then((module) => ({ default: module.SearchPage })),
);
const ProjectsPage = lazy(() =>
  import("../pages/dashboard/ProjectsPage").then((module) => ({ default: module.ProjectsPage })),
);
const ImportsPage = lazy(() =>
  import("../pages/imports/ImportsPage").then((module) => ({ default: module.ImportsPage })),
);
const SessionsPage = lazy(() =>
  import("../pages/sessions/SessionsPage").then((module) => ({ default: module.SessionsPage })),
);
const SettingsPage = lazy(() =>
  import("../pages/settings/SettingsPage").then((module) => ({ default: module.SettingsPage })),
);

function deferred(page: ReactNode) {
  return (
    <Suspense fallback={<div className="surface-block text-sm text-[var(--text-secondary)]">正在载入页面…</div>}>
      {page}
    </Suspense>
  );
}

export function App() {
  return (
    <ThemeProvider>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<Navigate to="/dashboard" replace />} />
          <Route path="/dashboard" element={deferred(<DashboardPage />)} />
          <Route path="/performance" element={deferred(<PerformancePage />)} />
          <Route path="/workflow" element={deferred(<WorkflowPage />)} />
          <Route path="/correlations" element={deferred(<CorrelationsPage />)} />
          <Route path="/search" element={deferred(<SearchPage />)} />
          <Route path="/projects" element={deferred(<ProjectsPage />)} />
          <Route path="/imports" element={deferred(<ImportsPage />)} />
          <Route path="/sessions" element={deferred(<SessionsPage />)} />
          <Route path="/settings" element={deferred(<SettingsPage />)} />
        </Route>
      </Routes>
    </ThemeProvider>
  );
}
