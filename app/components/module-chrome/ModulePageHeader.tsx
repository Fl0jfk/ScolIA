"use client";

import type { ReactNode } from "react";
import { dash } from "@/app/lib/dashboard-brand";

export default function ModulePageHeader({
  eyebrow,
  title,
  description,
  actions,
}: {
  eyebrow?: string;
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between mb-6">
      <div className="min-w-0">
        {eyebrow ? (
          <p className={`text-[10px] font-semibold uppercase tracking-[0.24em] ${dash.textMid}`}>
            {eyebrow}
          </p>
        ) : null}
        <h1 className={`mt-1 text-2xl sm:text-[1.75rem] font-semibold tracking-tight ${dash.ink}`}>
          {title}
        </h1>
        {description ? (
          <div className={`mt-1 text-sm ${dash.textMid}`}>{description}</div>
        ) : null}
      </div>
      {actions ? <div className="shrink-0 flex items-center gap-2">{actions}</div> : null}
    </header>
  );
}
