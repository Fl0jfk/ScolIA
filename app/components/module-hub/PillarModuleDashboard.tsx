"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import { dash } from "@/app/lib/dashboard-brand";
import {
  DASHBOARD_PILLARS,
  categoriesForPillar,
  type DashboardPillarId,
} from "@/app/lib/dashboard-pillars";
import type { DashboardCategory } from "@/app/lib/intranet-modules";
import type { DashboardShortcut } from "@/app/lib/dashboard-signals";
import { MODULE_EMOJI, moduleHref } from "@/app/lib/pillar-module-routes";

const PILLAR_ORB: Record<Exclude<DashboardPillarId, "rh">, string> = {
  eleves: "bg-sky-400/30",
  etablissement: "bg-amber-400/30",
  services: "bg-emerald-400/30",
};

type Props = {
  pillarId: Exclude<DashboardPillarId, "rh">;
  categories: DashboardCategory[];
  accessibleModuleIds: Set<string>;
};

type PreviewLine = {
  id: string;
  title: string;
  detail?: string;
  badge?: string;
  href: string;
};

function previewLinesForModule(
  moduleId: string,
  shortcuts: DashboardShortcut[],
  max = 6,
): PreviewLine[] {
  const related = shortcuts.filter((s) => s.moduleId === moduleId);
  // Priorité : aperçus pillarOnly, puis rich dynamiques — jamais les labels génériques seuls
  const dynamic = related.filter(
    (s) => s.pillarOnly || s.rich || Boolean(s.detail) || Boolean(s.badge),
  );
  // Sur le sous-dashboard, éviter le doublon agrégé « Cette semaine » si on a déjà les sorties unitaires
  const hasUnitTrips = dynamic.some((s) => s.id.startsWith("travels-up-"));
  const ordered = dynamic
    .filter((s) => !(hasUnitTrips && s.id === "travels-week"))
    .filter((s) => !(hasUnitTrips && s.id === "travels-today"))
    .sort((a, b) => {
      const score = (s: DashboardShortcut) =>
        (s.pillarOnly ? 4 : 0) + (s.rich ? 2 : 0) + (s.badge ? 1 : 0);
      return score(b) - score(a);
    });

  const lines: PreviewLine[] = [];
  const seen = new Set<string>();
  for (const s of ordered) {
    if (lines.length >= max) break;
    const key = `${s.label}|${s.detail || ""}|${s.badge || ""}`;
    if (seen.has(key)) continue;
    seen.add(key);
    // Skip pure module title without info
    if (!s.rich && !s.detail && !s.badge && !s.pillarOnly) continue;
    lines.push({
      id: s.id,
      title: s.label,
      detail: s.detail,
      badge: s.badge,
      href: s.href,
    });
  }
  return lines;
}

function ModuleCard({
  category,
  previews,
  index,
  orbClass,
}: {
  category: DashboardCategory;
  previews: PreviewLine[];
  index: number;
  orbClass: string;
}) {
  const href = category.link || moduleHref(category.moduleId);
  const emoji = MODULE_EMOJI[category.moduleId] || "›";

  return (
    <motion.article
      layout
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.45, delay: 0.04 * index, ease: [0.22, 1, 0.36, 1] }}
      className="group relative flex min-h-0 flex-col overflow-hidden rounded-[1.5rem] border border-white/55 bg-white/45 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-2xl"
    >
      <div
        className={`pointer-events-none absolute -right-10 -top-12 h-44 w-44 rounded-full blur-3xl transition duration-700 group-hover:scale-110 ${orbClass}`}
        aria-hidden
      />

      <div className="relative flex min-h-0 flex-1 flex-col p-4 sm:p-5">
        <header className="mb-3 flex shrink-0 items-center justify-between gap-3">
          <div className="min-w-0 flex items-center gap-3">
            <span className="flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-white/70 ring-1 ring-white/80 shadow-sm">
              {category.img ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={category.img} alt="" className="h-full w-full object-cover" />
              ) : (
                <span className="text-lg leading-none">{emoji}</span>
              )}
            </span>
            <h2 className="truncate text-lg font-semibold tracking-tight text-[var(--dash-ink)] sm:text-xl">
              {category.name}
            </h2>
          </div>
          <Link
            href={href}
            className="shrink-0 !cursor-pointer rounded-full border border-white/70 bg-white/70 px-3 py-1.5 text-[11px] font-semibold text-[var(--dash-primary)] shadow-sm backdrop-blur transition hover:bg-white"
            style={{ cursor: "pointer" }}
          >
            Ouvrir →
          </Link>
        </header>

        <div className="flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto">
          <AnimatePresence mode="popLayout">
            {previews.length === 0 ? (
              <p className="text-xs text-[var(--dash-mid)]">Rien à signaler pour le moment.</p>
            ) : (
              previews.map((line) => (
                <Link
                  key={line.id}
                  href={line.href || href}
                  className="flex items-start gap-2 rounded-xl border border-white/60 bg-white/55 px-2.5 py-2.5 transition hover:bg-white/85"
                >
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-[13px] font-semibold text-[var(--dash-ink)]">
                      {line.title}
                    </p>
                    {line.detail ? (
                      <p className="mt-0.5 line-clamp-2 text-[11px] leading-snug text-[var(--dash-mid)]">
                        {line.detail}
                      </p>
                    ) : null}
                  </div>
                  {line.badge ? (
                    <span className="shrink-0 rounded-full bg-[var(--dash-primary)] px-2 py-0.5 text-[9px] font-bold text-white">
                      {line.badge}
                    </span>
                  ) : null}
                </Link>
              ))
            )}
          </AnimatePresence>
        </div>
      </div>
    </motion.article>
  );
}

export default function PillarModuleDashboard({
  pillarId,
  categories,
  accessibleModuleIds,
}: Props) {
  const pillar = DASHBOARD_PILLARS.find((p) => p.id === pillarId)!;
  const [shortcuts, setShortcuts] = useState<DashboardShortcut[]>([]);
  const [loadingSignals, setLoadingSignals] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoadingSignals(true);
    fetch("/api/dashboard/signals", { cache: "no-store" })
      .then((r) => r.json())
      .then((j) => {
        if (cancelled) return;
        setShortcuts(Array.isArray(j.shortcuts) ? j.shortcuts : []);
      })
      .catch(() => {
        if (!cancelled) setShortcuts([]);
      })
      .finally(() => {
        if (!cancelled) setLoadingSignals(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const modules = useMemo(() => {
    return categoriesForPillar(pillar, categories).filter((c) =>
      accessibleModuleIds.has(c.moduleId),
    );
  }, [pillar, categories, accessibleModuleIds]);

  const pillarShortcuts = useMemo(
    () => shortcuts.filter((s) => s.pillarId === pillarId),
    [shortcuts, pillarId],
  );

  const orb = PILLAR_ORB[pillarId];
  const count = modules.length;
  const gridClass =
    count <= 2
      ? "grid-cols-1 sm:grid-cols-2"
      : count <= 4
        ? "grid-cols-1 sm:grid-cols-2"
        : "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3";

  return (
    <DashboardThemeRoot>
      <div className="relative flex min-h-[calc(100dvh-4.5rem)] flex-col">
        <div
          className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_top_left,color-mix(in_srgb,var(--dash-soft)_80%,transparent),transparent_55%),radial-gradient(ellipse_at_bottom_right,color-mix(in_srgb,var(--dash-bright)_18%,transparent),transparent_50%)]"
          aria-hidden
        />
        <main className="relative mx-auto flex w-full max-w-[90rem] flex-1 flex-col px-4 py-4 sm:px-6 sm:py-5 lg:px-8">
          <div className="relative mb-4 flex shrink-0 items-center justify-center">
            <Link
              href="/dashboard"
              className={`absolute left-0 text-xs font-bold tracking-wide ${dash.textPrimary} hover:underline`}
            >
              ← Tableau de bord
            </Link>
            <h1 className={`text-2xl font-black tracking-tight sm:text-3xl ${dash.ink}`}>
              {pillar.title}
            </h1>
          </div>

          {modules.length === 0 ? (
            <p className="rounded-2xl border border-white/60 bg-white/50 px-5 py-8 text-center text-sm text-[var(--dash-mid)] backdrop-blur">
              Aucun module accessible pour votre profil. Contactez un administrateur si besoin.
            </p>
          ) : (
            <div className={`grid min-h-0 flex-1 gap-3 sm:gap-4 ${gridClass} auto-rows-fr`}>
              {modules.map((cat, i) => (
                <ModuleCard
                  key={cat.moduleId}
                  category={cat}
                  index={i}
                  orbClass={orb}
                  previews={
                    loadingSignals
                      ? []
                      : previewLinesForModule(cat.moduleId, pillarShortcuts, 6)
                  }
                />
              ))}
            </div>
          )}
        </main>
      </div>
    </DashboardThemeRoot>
  );
}
