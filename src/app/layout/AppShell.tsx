import {
  ChartScatter,
  DatabaseZap,
  FolderKanban,
  FolderSearch2,
  Gauge,
  LayoutDashboard,
  SearchCode,
  Settings2,
  SunMoon,
  Workflow,
} from "lucide-react";
import clsx from "clsx";
import { NavLink, Outlet } from "react-router-dom";
import { type ThemeMode, useTheme } from "../theme";
import { useAutoImportSync } from "../../features/imports/autoSync";

const navItems = [
  { to: "/dashboard", label: "概览", icon: LayoutDashboard },
  { to: "/performance", label: "性能", icon: Gauge },
  { to: "/workflow", label: "工作流", icon: Workflow },
  { to: "/correlations", label: "相关性", icon: ChartScatter },
  { to: "/search", label: "搜索", icon: SearchCode },
  { to: "/projects", label: "项目", icon: FolderKanban },
  { to: "/imports", label: "导入", icon: FolderSearch2 },
  { to: "/sessions", label: "会话", icon: DatabaseZap },
  { to: "/settings", label: "设置", icon: Settings2 },
];

const themeOptions = [
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
  { value: "system", label: "系统" },
] as const;

function SidebarThemeSwitcher({ mode, setMode }: { mode: ThemeMode; setMode: (mode: ThemeMode) => void }) {
  return (
    <section className="sidebar__theme" aria-label="外观设置">
      <div className="sidebar__theme-header">
        <SunMoon className="h-4 w-4" />
        <span>外观</span>
      </div>
      <div className="sidebar__theme-group">
        {themeOptions.map((option) => (
          <button
            key={option.value}
            type="button"
            className={clsx("sidebar__theme-button", {
              "is-active": mode === option.value,
            })}
            onClick={() => setMode(option.value)}
          >
            {option.label}
          </button>
        ))}
      </div>
    </section>
  );
}

export function AppShell() {
  const { mode, setMode } = useTheme();
  useAutoImportSync();

  return (
    <div className="app-shell">
      <div className="app-shell__backdrop" />
      <div className="app-shell__inner">
        <aside className="sidebar">
          <div className="sidebar__brand">
            <p className="sidebar__eyebrow">Local First Analytics</p>
            <h1 className="sidebar__title">rescue_codex</h1>
          </div>

          <nav className="sidebar__nav">
            {navItems.map(({ to, label, icon: Icon }) => (
              <NavLink
                key={to}
                to={to}
                className={({ isActive }) =>
                  clsx("sidebar__link", {
                    "is-active": isActive,
                  })
                }
              >
                <Icon className="h-4 w-4" />
                <span>{label}</span>
              </NavLink>
            ))}
          </nav>

          <div className="sidebar__footer">
            <SidebarThemeSwitcher mode={mode} setMode={setMode} />
          </div>
        </aside>

        <main className="app-shell__content">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
