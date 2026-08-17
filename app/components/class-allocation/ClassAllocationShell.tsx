import type { ReactNode } from "react";
import Link from "next/link";

export function ClassAllocationShell({
  title,
  subtitle,
  badge,
  children,
  backHref,
}: {
  title: string;
  subtitle?: string;
  badge?: string;
  children: ReactNode;
  backHref?: string;
}) {
  return (
    <div className="min-h-screen bg-gradient-to-b from-slate-50 via-white to-indigo-50/40">
      <div className="border-b border-slate-200/80 bg-white/80 backdrop-blur">
        <div className="mx-auto flex max-w-[1280px] items-center justify-between gap-4 px-4 py-4">
          <div>
            {badge && (
              <p className="text-[10px] font-black uppercase tracking-[0.2em] text-indigo-600">{badge}</p>
            )}
            <h1 className="text-2xl font-black tracking-tight text-slate-900 sm:text-3xl">{title}</h1>
            {subtitle && <p className="mt-1 max-w-2xl text-sm text-slate-600">{subtitle}</p>}
          </div>
          {backHref && (
            <Link
              href={backHref}
              className="shrink-0 rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
            >
              ← Retour
            </Link>
          )}
        </div>
      </div>
      <main className="mx-auto max-w-[1280px] px-4 py-8">{children}</main>
    </div>
  );
}

export function ClassAllocationCard({
  title,
  description,
  children,
  className = "",
}: {
  title?: string;
  description?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`rounded-2xl border border-slate-200 bg-white p-5 shadow-sm ${className}`}>
      {title && <h2 className="text-base font-bold text-slate-900">{title}</h2>}
      {description && <p className="mt-1 text-sm text-slate-500">{description}</p>}
      <div className={title || description ? "mt-4 space-y-4" : "space-y-4"}>{children}</div>
    </section>
  );
}

export function ClassAllocationAlert({
  tone = "info",
  children,
}: {
  tone?: "info" | "warn" | "success" | "error";
  children: ReactNode;
}) {
  const styles = {
    info: "border-indigo-200 bg-indigo-50 text-indigo-900",
    warn: "border-amber-200 bg-amber-50 text-amber-900",
    success: "border-emerald-200 bg-emerald-50 text-emerald-900",
    error: "border-rose-200 bg-rose-50 text-rose-900",
  }[tone];
  return <div className={`rounded-xl border px-4 py-3 text-sm ${styles}`}>{children}</div>;
}
