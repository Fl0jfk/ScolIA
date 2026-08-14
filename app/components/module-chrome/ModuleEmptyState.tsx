"use client";

import type { ReactNode } from "react";
import { dash } from "@/app/lib/dashboard-brand";

export default function ModuleEmptyState({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`rounded-3xl border border-dashed p-10 text-center ${dash.border} ${dash.bgSoft50} ${dash.textMid} ${className}`}
    >
      {children}
    </div>
  );
}
