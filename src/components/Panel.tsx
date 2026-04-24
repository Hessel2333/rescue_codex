import { ReactNode } from "react";
import clsx from "clsx";
import { CircleHelp } from "lucide-react";

type PanelProps = {
  title: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
};

export function Panel({ title, description, actions, children, className }: PanelProps) {
  return (
    <section className={clsx("panel", className)}>
      <div className="mb-5 flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <div className="panel__title-row">
            <h3 className="section-title">{title}</h3>
            {description ? (
              <div className="panel__help">
                <button type="button" className="panel__help-button" aria-label={`${title} 说明`}>
                  <CircleHelp className="h-4 w-4" />
                </button>
                <div className="panel__tooltip">{description}</div>
              </div>
            ) : null}
          </div>
        </div>
        {actions}
      </div>
      {children}
    </section>
  );
}
