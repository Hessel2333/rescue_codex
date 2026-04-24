import { useMemo } from "react";
import { ActivityPoint } from "../../types/api";

type GitHubHeatmapProps = {
  data: ActivityPoint[];
};

type HeatmapCell = {
  date: string;
  questions: number;
  sessions: number;
  avgFirstResponseSec: number;
};

const weekdayLabels = ["周一", "", "周三", "", "周五", "", "周日"];
const monthFormatter = new Intl.DateTimeFormat("zh-CN", { month: "short" });
const fullDateFormatter = new Intl.DateTimeFormat("zh-CN", {
  year: "numeric",
  month: "long",
  day: "numeric",
  weekday: "long",
});

function parseDate(value: string) {
  const [year, month, day] = value.split("-").map(Number);
  return new Date(year, month - 1, day, 12, 0, 0, 0);
}

function startOfWeek(date: Date) {
  const copy = new Date(date);
  const current = copy.getDay();
  const delta = current === 0 ? -6 : 1 - current;
  copy.setDate(copy.getDate() + delta);
  return copy;
}

function toIsoDate(date: Date) {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function formatDuration(seconds: number) {
  if (!seconds) {
    return "0s";
  }
  if (seconds < 60) {
    return `${seconds}s`;
  }
  const minutes = Math.floor(seconds / 60);
  const remain = seconds % 60;
  return remain > 0 ? `${minutes}m ${remain}s` : `${minutes}m`;
}

function levelFor(value: number, maxValue: number) {
  if (value <= 0 || maxValue <= 0) {
    return 0;
  }
  const ratio = value / maxValue;
  if (ratio <= 0.25) {
    return 1;
  }
  if (ratio <= 0.5) {
    return 2;
  }
  if (ratio <= 0.75) {
    return 3;
  }
  return 4;
}

export function GitHubHeatmap({ data }: GitHubHeatmapProps) {
  const model = useMemo(() => {
    const map = new Map<string, HeatmapCell>();
    for (const item of data) {
      map.set(item.date, {
        date: item.date,
        questions: item.questions,
        sessions: item.sessions,
        avgFirstResponseSec: item.avgFirstResponseSec,
      });
    }

    const latestDate = data[data.length - 1]?.date ? parseDate(data[data.length - 1].date) : new Date();
    const endDate = latestDate;
    const startDate = startOfWeek(new Date(endDate.getFullYear(), endDate.getMonth(), endDate.getDate() - 364, 12, 0, 0, 0));
    const weeks: HeatmapCell[][] = [];
    const months: Array<{ label: string; weekIndex: number }> = [];
    const seenMonths = new Set<string>();
    const cursor = new Date(startDate);

    while (cursor <= endDate) {
      const week: HeatmapCell[] = [];
      for (let row = 0; row < 7; row += 1) {
        const current = new Date(cursor);
        current.setDate(cursor.getDate() + row);
        const iso = toIsoDate(current);
        const monthKey = `${current.getFullYear()}-${current.getMonth()}`;
        if ((weeks.length === 0 && row === 0) || (current.getDate() === 1 && !seenMonths.has(monthKey))) {
          months.push({
            label: monthFormatter.format(current),
            weekIndex: weeks.length,
          });
          seenMonths.add(monthKey);
        }
        week.push(
          map.get(iso) ?? {
            date: iso,
            questions: 0,
            sessions: 0,
            avgFirstResponseSec: 0,
          },
        );
      }

      weeks.push(week);
      cursor.setDate(cursor.getDate() + 7);
    }

    const maxQuestions = Math.max(0, ...data.map((item) => item.questions));
    return { weeks, months, maxQuestions };
  }, [data]);

  if (data.length === 0) {
    return (
      <div className="empty-state">
        当前还没有足够的日级活跃数据。
      </div>
    );
  }

  return (
    <div className="contribution-heatmap">
      <div
        className="contribution-heatmap__months"
        style={{ gridTemplateColumns: `repeat(${model.weeks.length}, minmax(10px, 1fr))` }}
      >
        {model.months.map((month) => (
          <span
            key={`${month.label}-${month.weekIndex}`}
            className="contribution-heatmap__month"
            style={{ gridColumn: month.weekIndex + 1 }}
          >
            {month.label}
          </span>
        ))}
      </div>

      <div className="contribution-heatmap__body">
        <div className="contribution-heatmap__weekdays">
          {weekdayLabels.map((label, index) => (
            <span key={`${label}-${index}`}>{label}</span>
          ))}
        </div>

        <div
          className="contribution-heatmap__grid"
          style={{ gridTemplateColumns: `repeat(${model.weeks.length}, minmax(10px, 1fr))` }}
        >
          {model.weeks.map((week, weekIndex) =>
            week.map((cell, rowIndex) => {
              const label = [
                fullDateFormatter.format(parseDate(cell.date)),
                `提问 ${cell.questions} 次`,
                `会话 ${cell.sessions} 个`,
                `平均首 token 耗时 ${formatDuration(cell.avgFirstResponseSec)}`,
              ].join("\n");

              return (
                <span
                  key={`${weekIndex}-${cell.date}`}
                  className={`contribution-heatmap__cell level-${levelFor(cell.questions, model.maxQuestions)}`}
                  style={{ gridColumn: weekIndex + 1, gridRow: rowIndex + 1 }}
                  title={label}
                />
              );
            }),
          )}
        </div>
      </div>

      <div className="contribution-heatmap__legend">
        <span>少</span>
        <div className="contribution-heatmap__legend-scale">
          {[0, 1, 2, 3, 4].map((value) => (
            <span key={value} className={`contribution-heatmap__cell level-${value}`} />
          ))}
        </div>
        <span>多</span>
      </div>
    </div>
  );
}
