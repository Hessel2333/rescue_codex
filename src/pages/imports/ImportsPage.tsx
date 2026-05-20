import { useEffect, useMemo, useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { ImportActions } from "../../features/imports/ImportActions";
import { ImportProgress, ImportStatusBadge } from "../../features/imports/ImportProgress";
import { getDashboardSummary, importPaths, pickFiles, pickFolders, scanDefaultSource } from "../../lib/tauri";
import { formatDateTime } from "../../lib/format";
import { DashboardSummary, ImportRunResult } from "../../types/api";

export function ImportsPage() {
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ImportRunResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const latestImport = summary?.recentImports[0];
  const importRunning = latestImport?.status === "running" || (!latestImport && result?.status === "running");
  const activeImport = useMemo(() => {
    if (latestImport?.status === "running") {
      return latestImport;
    }

    return result?.status === "running" ? result : latestImport;
  }, [latestImport, result]);

  async function load() {
    try {
      const payload = await getDashboardSummary({}, "imports");
      setSummary(payload);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "无法加载导入状态。");
    }
  }

  async function run(task: () => Promise<ImportRunResult | null>) {
    setBusy(true);
    setError(null);
    setNotice("正在启动导入任务…");

    try {
      const payload = await task();
      if (!payload) {
        setNotice("已取消选择。");
        return;
      }
      setResult(payload);
      setNotice(payload.status === "running" ? "后台导入已启动，进度会自动刷新。" : "导入任务已完成。");
      await load();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "导入失败。");
      setNotice(null);
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void load();
  }, []);

  useEffect(() => {
    if (!importRunning) {
      return;
    }

    const timer = window.setInterval(() => {
      void load();
    }, 2000);

    return () => window.clearInterval(timer);
  }, [importRunning]);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        eyebrow="Ingestion"
        title="本地扫描与导入"
        description="默认扫描 `~/.codex/sessions` 与归档会话，也支持手动导入 JSON / JSONL 文件或目录。导入会在后台执行。"
      />

      <Panel
        title="导入入口"
        description="导入时会自动选择最合适的解析器，进度会持续刷新，期间可以继续浏览其它页面。"
        actions={
          <ImportActions
            busy={busy || importRunning}
            onScanDefault={() => void run(() => scanDefaultSource())}
            onImportFiles={() =>
              void run(async () => {
                const paths = await pickFiles();
                if (paths.length === 0) {
                  return null;
                }
                return importPaths(paths);
              })
            }
            onImportFolders={() =>
              void run(async () => {
                const paths = await pickFolders();
                if (paths.length === 0) {
                  return null;
                }
                return importPaths(paths);
              })
            }
          />
        }
      >
        {notice ? (
          <div className="mb-4">
            <p className="action-status action-status--left">{notice}</p>
          </div>
        ) : null}

        {activeImport ? (
          <div className="import-run-card mb-4">
            <div className="min-w-0">
              <p className="meta-label">当前导入任务</p>
              <p className="section-title mt-2">{activeImport.sourceLabel}</p>
              {"rootPath" in activeImport ? <p className="mono-value mt-1 text-xs">{activeImport.rootPath}</p> : null}
            </div>
            <ImportProgress item={activeImport} />
          </div>
        ) : null}

        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
          <div className="surface-tile">
            <p className="meta-label">默认扫描根</p>
            <p className="mono-value mt-2">{summary?.appInfo.defaultCodexRoot ?? "尚未检测"}</p>
          </div>
          <div className="surface-tile">
            <p className="meta-label">数据库</p>
            <p className="mono-value mt-2">{summary?.appInfo.databasePath ?? "尚未创建"}</p>
          </div>
          <div className="surface-tile">
            <p className="meta-label">最近导入数</p>
            <p className="section-title mt-2">{summary?.recentImports.length ?? 0}</p>
          </div>
          <div className="surface-tile">
            <p className="meta-label">最近告警数</p>
            <p className="section-title mt-2">{summary?.recentIssues.length ?? 0}</p>
          </div>
        </div>
      </Panel>

      {result ? (
        <Panel title="最近一次执行" description="后台任务启动后，导入历史会持续显示进度。">
          <ImportProgress item={result} />
        </Panel>
      ) : null}

      {error ? <Panel title="执行失败">{error}</Panel> : null}

      <div className="grid gap-6 xl:grid-cols-2">
        <Panel title="导入历史" description="按最近时间排序，便于回看批量导入情况。">
          <div className="overflow-x-auto">
            <table className="data-table">
              <thead>
                <tr>
                  <th>来源</th>
                  <th>模式</th>
                  <th>状态</th>
                  <th>进度</th>
                  <th>开始时间</th>
                </tr>
              </thead>
              <tbody>
                {summary?.recentImports.map((item) => (
                  <tr key={item.id}>
                    <td>{item.sourceLabel}</td>
                    <td>{item.mode}</td>
                    <td>
                      <ImportStatusBadge status={item.status} />
                    </td>
                    <td>
                      <ImportProgress item={item} compact />
                    </td>
                    <td>{formatDateTime(item.startedAt)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Panel>

        <Panel title="异常与告警" description="坏行、空文件、缺失字段、未知事件类型都会记录在这里。">
          <div className="stack-list">
            {summary?.recentIssues.map((issue) => (
              <div key={issue.id} className="surface-block">
                <div className="warning-card__meta">
                  <span>{issue.severity}</span>
                  <span>{issue.code}</span>
                  <span>{formatDateTime(issue.createdAt)}</span>
                </div>
                <p className="body-text mt-2">{issue.message}</p>
                {issue.path ? <p className="mono-value mt-2 text-xs">{issue.path}</p> : null}
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  );
}
