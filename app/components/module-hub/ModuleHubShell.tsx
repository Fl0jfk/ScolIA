"use client";

import Link from "next/link";
import type { ReactNode } from "react";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import ModuleHubNav, { type ModuleHubTab } from "@/app/components/module-hub/ModuleHubNav";
import { dash } from "@/app/lib/dashboard-brand";

type Props = {
  title: string;
  description: string;
  tabs: ModuleHubTab[];
  active: string;
  onChange: (id: string) => void;
  children: ReactNode;
};

export default function ModuleHubShell({
  title,
  description,
  tabs,
  active,
  onChange,
  children,
}: Props) {
  return (
    <DashboardThemeRoot>
      <div className="relative min-h-[calc(100dvh-4.5rem)]">
        <div
          className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_top_left,color-mix(in_srgb,var(--dash-soft)_80%,transparent),transparent_55%),radial-gradient(ellipse_at_bottom_right,color-mix(in_srgb,var(--dash-bright)_18%,transparent),transparent_50%)]"
          aria-hidden
        />
        <main className="relative mx-auto max-w-7xl px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
          <div className="mb-5">
            <Link
              href="/dashboard"
              className={`text-xs font-bold tracking-wide ${dash.textPrimary} hover:underline`}
            >
              ← Tableau de bord
            </Link>
            <h1 className={`mt-2 text-3xl font-black tracking-tight sm:text-4xl ${dash.ink}`}>
              {title}
            </h1>
            <p className={`mt-1 max-w-2xl text-sm ${dash.textMid}`}>{description}</p>
          </div>

          <ModuleHubNav tabs={tabs} active={active} onChange={onChange} />

          <div className="rounded-3xl border border-[color:var(--dash-border)]/80 bg-white/70 p-3 shadow-[0_20px_60px_-40px_rgba(15,23,42,0.35)] backdrop-blur-xl sm:p-5">
            {children}
          </div>
        </main>
      </div>
    </DashboardThemeRoot>
  );
}
