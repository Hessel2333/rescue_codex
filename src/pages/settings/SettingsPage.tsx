import { useEffect, useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import { Panel } from "../../components/Panel";
import { getDashboardSummary } from "../../lib/tauri";
import { DashboardSummary } from "../../types/api";

export function SettingsPage() {
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getDashboardSummary({}, "settings")
      .then(setSummary)
      .catch((cause) => {
        setError(cause instanceof Error ? cause.message : "无法读取设置。");
      });
  }, []);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        eyebrow="Settings"
        title="应用设置与运行信息"
        description="查看本地数据路径与运行能力。后续可继续扩展默认扫描目录、导出偏好和实验功能开关。"
      />

      {error ? <Panel title="读取失败">{error}</Panel> : null}

      <Panel title="本地路径" description="所有数据都在本地读取和存储，不依赖逆向私有 API。">
        <div className="stack-list">
          <div className="surface-tile">
            <p className="meta-label">Codex 根目录</p>
            <p className="mono-value mt-2">{summary?.appInfo.defaultCodexRoot ?? "未检测"}</p>
          </div>
          <div className="surface-tile">
            <p className="meta-label">Session Index</p>
            <p className="mono-value mt-2">{summary?.appInfo.sessionIndexPath ?? "未检测"}</p>
          </div>
          <div className="surface-tile">
            <p className="meta-label">SQLite 数据库</p>
            <p className="mono-value mt-2">{summary?.appInfo.databasePath ?? "未创建"}</p>
          </div>
        </div>
      </Panel>

      <Panel title="当前能力" description="第一阶段已经打通的功能模块。">
        <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {[
            "默认扫描 ~/.codex/sessions",
            "手动导入 JSON / JSONL",
            "容错解析与重复导入去重",
            "原始事件 + 归一化消息双层存储",
            "概览 / 性能 / 工作流 / 搜索 四类分析",
            "CSV / JSON / Markdown 导出",
          ].map((item) => (
            <div key={item} className="capability-card">
              {item}
            </div>
          ))}
        </div>
      </Panel>
    </div>
  );
}
