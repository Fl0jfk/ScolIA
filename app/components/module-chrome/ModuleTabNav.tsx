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
            className={`shrink-0 flex items-center gap-1.5 px-4 py-2 rounded-xl text-sm font-bold transition-colors ${
              isActive
                ? `${dash.bgPrimary} text-white shadow-md`
                : `bg-white ${dash.ink} border ${dash.border} ${dash.hoverBorder}`
            }`}
          >
            {tab.icon ? <span>{tab.icon}</span> : null}
            <span>{tab.label}</span>
            {badge != null && badge > 0 ? (
              <span
                className={`ml-0.5 min-w-[1.25rem] h-5 px-1 rounded-full text-[10px] font-black flex items-center justify-center ${
                  isActive ? "bg-white/25 text-white" : "bg-amber-100 text-amber-800"
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
