"use client";

import type { ButtonHTMLAttributes, ReactNode } from "react";
import { dash } from "@/app/lib/dashboard-brand";

const VARIANTS = {
  primary: `${dash.btnPrimary} px-4 py-2.5 text-sm`,
  secondary: `rounded-xl border bg-white px-4 py-2.5 text-sm font-bold ${dash.border} ${dash.ink} ${dash.hoverBorder} disabled:opacity-50`,
  danger:
    "rounded-xl border border-red-200 bg-white px-4 py-2.5 text-sm font-bold text-red-700 hover:bg-red-50 disabled:opacity-50",
} as const;

export default function ModuleButton({
  variant = "primary",
  className = "",
  children,
  ...rest
}: {
  variant?: keyof typeof VARIANTS;
  children: ReactNode;
} & ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button type="button" className={`${VARIANTS[variant]} ${className}`} {...rest}>
      {children}
    </button>
  );
}
