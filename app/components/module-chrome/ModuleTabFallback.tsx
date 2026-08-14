"use client";

import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import { dash } from "@/app/lib/dashboard-brand";

export default function ModuleTabFallback() {
  return (
    <ModuleCard className="p-8 flex items-center justify-center gap-3">
      <span
        className={`h-5 w-5 rounded-full border-2 border-t-transparent animate-spin ${dash.spinner}`}
      />
      <span className={`text-sm font-bold ${dash.textMid}`}>Chargement…</span>
    </ModuleCard>
  );
}
