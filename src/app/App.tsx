import { Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./layout/AppShell";
import { ThemeProvider } from "./theme";
import { DashboardPage } from "../pages/dashboard/DashboardPage";
import { CorrelationsPage } from "../pages/dashboard/CorrelationsPage";
import { PerformancePage } from "../pages/dashboard/PerformancePage";
import { WorkflowPage } from "../pages/dashboard/WorkflowPage";
import { SearchPage } from "../pages/dashboard/SearchPage";
import { ProjectsPage } from "../pages/dashboard/ProjectsPage";
import { ImportsPage } from "../pages/imports/ImportsPage";
import { SessionsPage } from "../pages/sessions/SessionsPage";
import { SettingsPage } from "../pages/settings/SettingsPage";

export function App() {
  return (
    <ThemeProvider>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<Navigate to="/dashboard" replace />} />
          <Route path="/dashboard" element={<DashboardPage />} />
          <Route path="/performance" element={<PerformancePage />} />
          <Route path="/workflow" element={<WorkflowPage />} />
          <Route path="/correlations" element={<CorrelationsPage />} />
          <Route path="/search" element={<SearchPage />} />
          <Route path="/projects" element={<ProjectsPage />} />
          <Route path="/imports" element={<ImportsPage />} />
          <Route path="/sessions" element={<SessionsPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </ThemeProvider>
  );
}
