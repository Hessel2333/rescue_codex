import { useEffect, useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { ImportActions } from "../../features/imports/ImportActions";
import { getDashboardSummary, importPaths, pickFiles, pickFolders, scanDefaultSource } from "../../lib/tauri";
import { formatDateTime } from "../../lib/format";
import { DashboardSummary, ImportRunResult } from "../../types/api";

export function ImportsPage() {
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ImportRunResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    try {
      const payload = await getDashboardSummary();
      setSummary(payload);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "无法加载导入状态。");
    }
  }

  async function run(task: () => Promise<ImportRunResult>) {
    setBusy(true);
    setError(null);

    try {
      const payload = await task();
      setResult(payload);
      await load();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "导入失败。");
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void load();
  }, []);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        eyebrow="Ingestion"
        title="本地扫描与导入"
        description="默认扫描 `~/.codex/sessions`，也支持手动导入 JSON / JSONL 文件或目录。单文件异常不会中断整批导入。"
      />

      <Panel
        title="导入入口"
        description="导入时会自动选择最合适的解析器，原始事件和归一化消息会一起写入 SQLite。"
        actions={
          <ImportActions
            busy={busy}
            onScanDefault={() => void run(() => scanDefaultSource())}
            onImportFiles={() =>
              void run(async () => {
                const paths = await pickFiles();
                return importPaths(paths);
              })
            }
            onImportFolders={() =>
              void run(async () => {
                const paths = await pickFolders();
                return importPaths(paths);
              })
            }
          />
        }
      >
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
        <Panel title="最近一次执行" description="导入完成后，这里会展示本轮结果和问题摘要。">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
            <div className="surface-tile">
              <p className="meta-label">状态</p>
              <p className="section-title mt-2">{result.status}</p>
            </div>
            <div className="surface-tile">
              <p className="meta-label">文件</p>
              <p className="section-title mt-2">{`${result.filesSuccess}/${result.filesTotal}`}</p>
            </div>
            <div className="surface-tile">
              <p className="meta-label">失败</p>
              <p className="section-title mt-2">{result.filesFailed}</p>
            </div>
            <div className="surface-tile">
              <p className="meta-label">Warnings</p>
              <p className="section-title mt-2">{result.warningsCount}</p>
            </div>
            <div className="surface-tile">
              <p className="meta-label">Errors</p>
              <p className="section-title mt-2">{result.errorsCount}</p>
            </div>
          </div>
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
                  <th>开始时间</th>
                </tr>
              </thead>
              <tbody>
                {summary?.recentImports.map((item) => (
                  <tr key={item.id}>
                    <td>{item.sourceLabel}</td>
                    <td>{item.mode}</td>
                    <td>{item.status}</td>
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
