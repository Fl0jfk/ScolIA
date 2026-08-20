"use client";

import type { ReactNode } from "react";
import { AnimatePresence, motion } from "framer-motion";
import GlassLayer from "@/app/components/GlassLayer";
import { SettingsAtmosphere } from "@/app/components/settings/SettingsChrome";
import { dash, dashboardBrandStyle } from "@/app/lib/dashboard-brand";
import {
  ONBOARDING_CHAPTERS,
  TOTAL_CHAPTERS,
} from "@/app/lib/onboarding-chapters";
import { MARKETING } from "@/app/lib/marketing-site";

type Props = {
  chapter: number;
  reviewMode?: boolean;
  accent?: string | null;
  error?: string | null;
  banner?: ReactNode;
  children: ReactNode;
  footer: ReactNode;
};

export default function OnboardingShell({
  chapter,
  reviewMode,
  accent,
  error,
  banner,
  children,
  footer,
}: Props) {
  const meta = ONBOARDING_CHAPTERS[Math.min(TOTAL_CHAPTERS, Math.max(1, chapter)) - 1];
  const progress = (chapter / TOTAL_CHAPTERS) * 100;

  return (
    <div className="dashboard-themed relative min-h-screen overflow-hidden" style={dashboardBrandStyle(accent)}>
      <SettingsAtmosphere />
      <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-white/40 via-transparent to-[color:var(--dash-soft-muted)]/80" />

      <div className="relative z-[1] mx-auto flex min-h-screen max-w-3xl flex-col px-4 py-8 sm:px-6 sm:py-12">
        <header className="mb-8 flex items-start justify-between gap-4">
          <div>
            <p className={`text-[10px] font-bold uppercase tracking-[0.28em] ${dash.label}`}>
              {MARKETING.productName}
              {reviewMode ? " · Relecture" : " · Configuration"}
            </p>
            <p className={`mt-2 text-xs font-semibold ${dash.textMid}`}>
              Chapitre {chapter} sur {TOTAL_CHAPTERS}
            </p>
          </div>
          <div className="hidden sm:flex gap-1.5 pt-1">
            {ONBOARDING_CHAPTERS.map((c) => (
              <span
                key={c.id}
                className={`h-1.5 w-7 rounded-full transition-colors ${
                  c.id <= chapter ? "bg-[var(--dash-primary)]" : "bg-white/60"
                }`}
                title={c.eyebrow}
              />
            ))}
          </div>
        </header>

        <div className="mb-6 h-1.5 overflow-hidden rounded-full bg-white/50 shadow-inner">
          <motion.div
            className="h-full rounded-full bg-gradient-to-r from-[var(--dash-primary)] to-[var(--dash-bright)]"
            initial={false}
            animate={{ width: `${progress}%` }}
            transition={{ type: "spring", stiffness: 120, damping: 20 }}
          />
        </div>

        <AnimatePresence mode="wait">
          <motion.div
            key={chapter}
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
            className="flex flex-1 flex-col"
          >
            <div className="mb-6">
              <p className={`text-[10px] font-bold uppercase tracking-[0.24em] ${dash.label}`}>
                {meta.eyebrow}
              </p>
              <h1 className={`mt-2 text-3xl font-semibold tracking-tight sm:text-4xl ${dash.gradientText}`}>
                {meta.title}
              </h1>
              <p className={`mt-3 max-w-xl text-sm leading-relaxed sm:text-base ${dash.textMid}`}>
                {meta.subtitle}
              </p>
            </div>

            <section className="relative flex-1 overflow-hidden rounded-[1.5rem] border border-white/55 shadow-[0_24px_60px_-36px_rgba(15,23,42,0.5)]">
              <GlassLayer className="bg-white/55 backdrop-blur-2xl" />
              <div className="relative z-[1] p-5 sm:p-8">
                {error ? (
                  <div className="mb-4 rounded-2xl border border-red-200 bg-red-50/90 px-4 py-3 text-sm text-red-800">
                    {error}
                  </div>
                ) : null}
                {banner}
                {children}
              </div>
            </section>

            <div className="mt-6 flex flex-wrap items-center justify-between gap-3 pb-4">{footer}</div>
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}

export function OnboardingField({
  label,
  children,
  hint,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
}) {
  return (
    <label className="mb-4 block">
      <span className={dash.fieldLabel}>{label}</span>
      <div className="mt-1.5">{children}</div>
      {hint ? <span className={`mt-1.5 block text-xs ${dash.textMid}`}>{hint}</span> : null}
    </label>
  );
}

export const onboardingInputClass = dash.field;
