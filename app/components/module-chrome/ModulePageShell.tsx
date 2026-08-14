"use client";

import type { ReactNode } from "react";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";

export default function ModulePageShell({
  children,
  maxWidthClass = "max-w-[1200px]",
  className = "",
}: {
  children: ReactNode;
  maxWidthClass?: string;
  className?: string;
}) {
  return (
    <DashboardThemeRoot>
      <main className={`mx-auto w-full ${maxWidthClass} px-4 sm:px-6 py-6 sm:py-8 ${className}`}>
        {children}
      </main>
    </DashboardThemeRoot>
  );
}
