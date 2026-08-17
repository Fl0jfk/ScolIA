"use client";

import type { ReactNode } from "react";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import ReplayModuleTourButton from "@/app/components/module-tour/ReplayModuleTourButton";

export default function ModulePageShell({
  children,
  maxWidthClass = "max-w-[1400px]",
  className = "",
  tourModuleId,
}: {
  children: ReactNode;
  maxWidthClass?: string;
  className?: string;
  tourModuleId?: string;
}) {
  return (
    <DashboardThemeRoot>
      <main className={`mx-auto w-full ${maxWidthClass} px-4 sm:px-6 py-6 sm:py-8 ${className}`}>
        {children}
        {tourModuleId ? (
          <div className="mt-12 border-t border-slate-200/70 pt-4">
            <ReplayModuleTourButton moduleId={tourModuleId} />
          </div>
        ) : null}
      </main>
    </DashboardThemeRoot>
  );
}
