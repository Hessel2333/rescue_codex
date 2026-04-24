import { ReactNode } from "react";

type StatCardProps = {
  label: string;
  value: string;
  hint: string;
  icon?: ReactNode;
};

export function StatCard({ label, value, hint, icon }: StatCardProps) {
  return (
    <div className="metric-card">
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="meta-label">{label}</p>
          <p className="metric-card__value">{value}</p>
        </div>
        {icon ? <div className="metric-card__icon">{icon}</div> : null}
      </div>
      <p className="body-text mt-5">{hint}</p>
    </div>
  );
}
