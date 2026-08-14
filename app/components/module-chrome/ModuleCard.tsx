"use client";

import type { HTMLAttributes, ReactNode } from "react";
import { dash } from "@/app/lib/dashboard-brand";

export default function ModuleCard({
  children,
  className = "",
  ...rest
}: { children: ReactNode } & HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={`rounded-2xl border bg-white ${dash.border} ${className}`}
      {...rest}
    >
      {children}
    </div>
  );
}
