"use client";

import type { ReactNode } from "react";
import { motion } from "framer-motion";
import { dash } from "@/app/lib/dashboard-brand";

export const settingsInputClass =
  "mt-1.5 w-full rounded-2xl border border-white/70 bg-white/70 px-3.5 py-2.5 text-sm text-slate-800 shadow-[inset_0_1px_0_rgba(255,255,255,0.8)] outline-none backdrop-blur-sm transition placeholder:text-slate-400 focus:border-[color:var(--dash-mid)]/50 focus:ring-2 focus:ring-[color:var(--dash-soft)]";

export const settingsSelectClass = `${settingsInputClass} appearance-none`;

export const settingsPillClass =
  "inline-flex cursor-pointer items-center rounded-full border border-white/70 bg-white/70 px-3.5 py-1.5 text-[11px] font-semibold text-[var(--dash-primary)] shadow-sm backdrop-blur transition hover:bg-white";

export function SettingsAtmosphere() {
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
      <div className="absolute -left-24 -top-28 h-[22rem] w-[22rem] rounded-full bg-[color:var(--dash-soft)]/80 blur-3xl" />
      <div className="absolute right-[-4rem] top-16 h-80 w-80 rounded-full bg-[color:var(--dash-bright)]/25 blur-3xl" />
      <div className="absolute bottom-10 left-1/3 h-56 w-56 rounded-full bg-[color:var(--dash-mid)]/15 blur-3xl" />
      <motion.div
        className="absolute left-1/2 top-8 h-36 w-36 -translate-x-1/2 rounded-full bg-white/40 blur-2xl"
        animate={{ opacity: [0.35, 0.55, 0.35], scale: [1, 1.08, 1] }}
        transition={{ duration: 8, repeat: Infinity, ease: "easeInOut" }}
      />
    </div>
  );
}

export function SettingsSection({
  title,
  description,
  icon,
  children,
  footer,
  className = "",
}: {
  title?: string;
  description?: ReactNode;
  icon?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={`relative overflow-hidden rounded-[1.5rem] border border-white/55 bg-white/55 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-2xl ${className}`}
    >
      <div className="pointer-events-none absolute -right-10 -top-12 h-36 w-36 rounded-full bg-[color:var(--dash-soft)]/70 blur-3xl" />
      <div className="relative space-y-4 p-5 sm:p-6">
        {(title || description) && (
          <header>
            <div className="flex items-center gap-2.5">
              {icon ? (
                <span className="flex h-10 w-10 items-center justify-center rounded-2xl bg-white/70 text-lg shadow-sm ring-1 ring-white/80">
                  {icon}
                </span>
              ) : null}
              {title ? (
                <h2 className={`text-lg font-semibold tracking-tight ${dash.ink}`}>{title}</h2>
              ) : null}
            </div>
            {description ? <div className={`mt-1.5 text-sm leading-relaxed ${dash.textMid}`}>{description}</div> : null}
          </header>
        )}
        {children}
        {footer}
      </div>
    </section>
  );
}

export function SettingsField({
  label,
  hint,
  children,
  className = "",
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <label className={`block ${className}`}>
      <span className={`block text-[11px] font-semibold uppercase tracking-[0.14em] ${dash.textMid}`}>{label}</span>
      {hint ? <span className={`mt-0.5 block text-xs font-normal normal-case tracking-normal ${dash.textMid}`}>{hint}</span> : null}
      {children}
    </label>
  );
}

export function SettingsNotice({
  tone,
  children,
}: {
  tone: "ok" | "warn" | "info" | "error";
  children: ReactNode;
}) {
  const cls =
    tone === "ok"
      ? "border-emerald-200/80 bg-emerald-50/90 text-emerald-800"
      : tone === "warn"
        ? "border-amber-200/80 bg-amber-50/90 text-amber-950"
        : tone === "error"
          ? "border-rose-200/80 bg-rose-50/90 text-rose-800"
          : "border-white/70 bg-white/60 text-slate-700";
  return <div className={`rounded-2xl border px-4 py-3 text-sm leading-relaxed ${cls}`}>{children}</div>;
}

export function SettingsSaveButton({
  saving,
  disabled,
  label = "Enregistrer",
  savingLabel = "Enregistrement…",
}: {
  saving: boolean;
  disabled?: boolean;
  label?: string;
  savingLabel?: string;
}) {
  return (
    <button
      type="submit"
      disabled={saving || disabled}
      className="inline-flex items-center justify-center rounded-2xl bg-[color:var(--dash-primary)] px-5 py-2.5 text-sm font-semibold text-white shadow-[0_12px_28px_-16px_rgba(15,23,42,0.55)] transition hover:opacity-95 disabled:opacity-50"
    >
      {saving ? savingLabel : label}
    </button>
  );
}

export function SettingsLoading({ label = "Chargement…" }: { label?: string }) {
  return (
    <div className="flex items-center gap-3 py-10">
      <span className="h-5 w-5 animate-spin rounded-full border-2 border-[color:var(--dash-mid)]/30 border-t-[color:var(--dash-primary)]" />
      <p className={`text-sm ${dash.textMid}`}>{label}</p>
    </div>
  );
}
