"use client";

import { dash } from "@/app/lib/dashboard-brand";

export type ModuleTabItem<T extends string> = {
  id: T;
  label: string;
  icon?: string;
  hidden?: boolean;
  dataAttrs?: Record<string, string>;
};

export default function ModuleTabNav<T extends string>({
  tabs,
  active,
  onChange,
  badges,
  scroll = false,
  className = "",
  navDataTour,
}: {
  tabs: ModuleTabItem<T>[];
  active: T;
  onChange: (id: T) => void;
  badges?: Partial<Record<T, number>>;
  scroll?: boolean;
  className?: string;
  navDataTour?: string;
}) {
  const visible = tabs.filter((t) => !t.hidden);
  return (
    <nav
      data-tour={navDataTour}
      className={
        scroll
          ? `flex gap-1 overflow-x-auto pb-1 -mx-1 px-1 scrollbar-thin ${className}`
          : `flex flex-wrap gap-2 ${className}`
      }
    >
      {visible.map((tab) => {
        const isActive = active === tab.id;
        const badge = badges?.[tab.id];
        return (
          <button
            key={tab.id}
            type="button"
            onClick={() => onChange(tab.id)}
            {...tab.dataAttrs}
            className={`shrink-0 flex items-center gap-1.5 rounded-full px-4 py-2 text-sm font-bold transition-colors ${
              isActive
                ? "bg-[var(--dash-ink)] text-white shadow-md"
                : `bg-white ${dash.ink} border border-black/8 ${dash.hoverBorder}`
            }`}
          >
            {tab.icon ? <span>{tab.icon}</span> : null}
            <span>{tab.label}</span>
            {badge != null && badge > 0 ? (
              <span
                className={`ml-0.5 flex h-5 min-w-[1.25rem] items-center justify-center rounded-full px-1 text-[10px] font-black ${
                  isActive
                    ? "bg-[var(--dash-lime)] text-[var(--dash-ink)]"
                    : "bg-[color:var(--dash-lime)]/80 text-[var(--dash-ink)]"
                }`}
              >
                {badge}
              </span>
            ) : null}
          </button>
        );
      })}
    </nav>
  );
}
