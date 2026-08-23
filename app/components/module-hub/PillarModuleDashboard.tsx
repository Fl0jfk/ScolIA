"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useMemo, useRef, useState, type DragEvent } from "react";
import { AnimatePresence, motion } from "framer-motion";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import GlassLayer from "@/app/components/GlassLayer";
import { dash } from "@/app/lib/dashboard-brand";
import {
  DASHBOARD_PILLARS,
  categoriesForPillar,
  type DashboardPillarId,
} from "@/app/lib/dashboard-pillars";
import { stageDashboardUpload } from "@/app/lib/dashboard-upload-bridge";
import type { DashboardCategory } from "@/app/lib/intranet-modules";
import type { DashboardShortcut } from "@/app/lib/dashboard-signals";
import { notificationCountForModule } from "@/app/lib/dashboard-signals";
import { useDashboardSignals } from "@/app/hooks/useDashboardSignals";
import { MODULE_EMOJI, moduleHref } from "@/app/lib/pillar-module-routes";
import NotificationCountBadge from "@/app/components/Dashboard/NotificationCountBadge";

const PILLAR_ORB: Record<DashboardPillarId, string> = {
  administratif: "bg-sky-400/30",
  services: "bg-teal-400/30",
  vie_scolaire: "bg-emerald-400/30",
  notes: "bg-violet-400/25",
  compta_rh: "bg-amber-400/30",
  sante: "bg-rose-400/30",
};

type Props = {
  pillarId: DashboardPillarId;
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
  const dynamic = related.filter(
    (s) => s.pillarOnly || s.rich || Boolean(s.detail) || Boolean(s.badge),
  );
  const hasUnitTrips = dynamic.some((s) => s.id.startsWith("travels-up-"));
  const ordered = dynamic
    .filter((s) => !(hasUnitTrips && (s.id === "travels-week" || s.id === "travels-today")))
    .sort((a, b) => {
      const score = (s: DashboardShortcut) => {
        const tone =
          s.tone === "warn" ? 6 : s.tone === "action" ? 5 : s.tone === "info" ? 3 : 1;
        return (s.pillarOnly ? 2 : 0) + (s.rich ? 2 : 0) + tone;
      };
      return score(b) - score(a);
    });

  const lines: PreviewLine[] = [];
  const seen = new Set<string>();
  for (const s of ordered) {
    if (lines.length >= max) break;
    const key = `${s.label}|${s.detail || ""}|${s.badge || ""}`;
    if (seen.has(key)) continue;
    seen.add(key);
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

function OcrQuickDrop() {
  const router = useRouter();
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);
  const [hint, setHint] = useState<string | null>(null);

  const goWithFiles = useCallback(
    (files: FileList | File[] | null) => {
      if (!files || (Array.isArray(files) ? files.length === 0 : files.length === 0)) return;
      const ok = stageDashboardUpload("standard", files);
      if (!ok) {
        setHint("PDF uniquement");
        return;
      }
      setHint(null);
      router.push("/agentIAOCR?upload=1");
    },
    [router],
  );

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    setDragging(false);
    goWithFiles(e.dataTransfer.files);
  };

  return (
    <div className="mt-auto space-y-2 pt-2">
      <button
        type="button"
        onDragOver={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={onDrop}
        onClick={() => inputRef.current?.click()}
        className={`w-full rounded-xl border-2 border-dashed px-3 py-4 text-center transition ${
          dragging
            ? "border-[var(--dash-primary)] bg-white/90"
            : "border-white/70 bg-white/50 hover:bg-white/75"
        }`}
      >
        <p className="text-xs font-semibold text-[var(--dash-ink)]">Déposer un PDF ici</p>
        <p className="mt-0.5 text-[10px] text-[var(--dash-mid)]">ou cliquer pour choisir</p>
      </button>
      <input
        ref={inputRef}
        type="file"
        accept="application/pdf,.pdf"
        multiple
        className="hidden"
        onChange={(e) => {
          goWithFiles(e.target.files);
          e.target.value = "";
        }}
      />
      {hint ? <p className="text-[10px] text-amber-700">{hint}</p> : null}
    </div>
  );
}

function ModuleQuickActions({ moduleId }: { moduleId: string }) {
  if (moduleId === "travels") {
    return (
      <div className="mt-auto flex flex-wrap gap-2 pt-3">
        <Link
          href="/travels/simple"
          className="rounded-full bg-[var(--dash-primary)] px-3 py-1.5 text-[11px] font-bold text-white shadow-sm hover:brightness-110"
        >
          + Sortie simple
        </Link>
        <Link
          href="/travels/complex"
          className="rounded-full border border-white/70 bg-white/70 px-3 py-1.5 text-[11px] font-semibold text-[var(--dash-primary)] hover:bg-white"
        >
          + Voyage / bus
        </Link>
      </div>
    );
  }
  if (moduleId === "internat") {
    return (
      <div className="mt-auto flex flex-wrap gap-2 pt-3">
        <Link
          href="/gestion-internat"
          className="rounded-full bg-[var(--dash-primary)] px-3 py-1.5 text-[11px] font-bold text-white shadow-sm hover:brightness-110"
        >
          Ouvrir l’appel
        </Link>
      </div>
    );
  }
  if (moduleId === "stages") {
    return (
      <div className="mt-auto flex flex-wrap gap-2 pt-3">
        <Link
          href="/stages"
          className="rounded-full border border-white/70 bg-white/70 px-3 py-1.5 text-[11px] font-semibold text-[var(--dash-primary)] hover:bg-white"
        >
          Voir les conventions
        </Link>
      </div>
    );
  }
  if (moduleId === "agent-ia-ocr") {
    return <OcrQuickDrop />;
  }
  if (moduleId === "requests-staff") {
    return (
      <div className="mt-auto flex flex-wrap gap-2 pt-3">
        <Link
          href="/faire-une-demande"
          className="rounded-full bg-[var(--dash-primary)] px-3 py-1.5 text-[11px] font-bold text-white shadow-sm hover:brightness-110"
        >
          + Nouvelle demande
        </Link>
        <Link
          href="/requests"
          className="rounded-full border border-white/70 bg-white/70 px-3 py-1.5 text-[11px] font-semibold text-[var(--dash-primary)] hover:bg-white"
        >
          File
        </Link>
      </div>
    );
  }
  if (moduleId === "prof-room") {
    return (
      <div className="mt-auto flex flex-wrap gap-2 pt-3">
        <Link
          href="/prof-room"
          className="rounded-full bg-[var(--dash-primary)] px-3 py-1.5 text-[11px] font-bold text-white shadow-sm hover:brightness-110"
        >
          Réserver une salle
        </Link>
      </div>
    );
  }
  if (moduleId === "photocopies-couleur") {
    return (
      <div className="mt-auto flex flex-wrap gap-2 pt-3">
        <Link
          href="/photocopies-couleur"
          className="rounded-full border border-white/70 bg-white/70 px-3 py-1.5 text-[11px] font-semibold text-[var(--dash-primary)] hover:bg-white"
        >
          Nouvelle demande
        </Link>
      </div>
    );
  }
  return null;
}

function ModuleCard({
  category,
  previews,
  index,
  orbClass,
  notifCount = 0,
}: {
  category: DashboardCategory;
  previews: PreviewLine[];
  index: number;
  orbClass: string;
  notifCount?: number;
}) {
  const href = category.link || moduleHref(category.moduleId);
  const emoji = MODULE_EMOJI[category.moduleId] || "›";

  return (
    <motion.article
      layout
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.45, delay: 0.04 * index, ease: [0.22, 1, 0.36, 1] }}
      className="group relative flex min-h-0 flex-col overflow-hidden rounded-[1.5rem] border border-white/55 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)]"
    >
      <GlassLayer className="bg-white/45 backdrop-blur-2xl" />
      <div
        className={`pointer-events-none absolute -right-10 -top-12 z-0 h-44 w-44 rounded-full blur-3xl transition duration-700 group-hover:scale-110 ${orbClass}`}
        aria-hidden
      />

      <div className="relative z-[1] flex min-h-0 flex-1 flex-col p-4 sm:p-5">
        <header className="mb-3 flex shrink-0 items-center justify-between gap-3">
          <div className="min-w-0 flex items-center gap-3">
            <span className="flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-white/70 ring-1 ring-white/80 shadow-sm">
              <span className="text-lg leading-none" aria-hidden>
                {emoji}
              </span>
            </span>
            <div className="min-w-0 flex items-center gap-2">
              <h2 className="truncate text-lg font-semibold tracking-tight text-[var(--dash-ink)] sm:text-xl">
                {category.name}
              </h2>
              <NotificationCountBadge count={notifCount} />
            </div>
          </div>
          <Link
            href={href}
            className="shrink-0 cursor-pointer rounded-full border border-white/70 bg-white/80 px-3 py-1.5 text-[11px] font-semibold text-[var(--dash-primary)] shadow-sm transition hover:bg-white"
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
                  className="flex cursor-pointer items-start gap-2 rounded-xl border border-white/60 bg-white/55 px-2.5 py-2.5 transition hover:bg-white/85"
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

        <ModuleQuickActions moduleId={category.moduleId} />
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
  const { shortcuts, notifications, loading: loadingSignals } = useDashboardSignals({
    pollIntervalMs: 0,
  });

  const modules = useMemo(() => {
    return categoriesForPillar(pillar, categories).filter((c) =>
      accessibleModuleIds.has(c.moduleId),
    );
  }, [pillar, categories, accessibleModuleIds]);

  const pillarShortcuts = useMemo(
    () => shortcuts.filter((s) => s.pillarId === pillarId),
    [shortcuts, pillarId],
  );

  /** Modules avec activité en premier (warn / action / info). */
  const orderedModules = useMemo(() => {
    const score = (moduleId: string) => {
      const related = pillarShortcuts.filter((s) => s.moduleId === moduleId && s.rich);
      let best = 10;
      for (const s of related) {
        if (s.tone === "warn") best = Math.min(best, 0);
        else if (s.tone === "action") best = Math.min(best, 1);
        else if (s.tone === "info") best = Math.min(best, 2);
        else if (s.tone !== "neutral") best = Math.min(best, 3);
      }
      return best;
    };
    return [...modules].sort((a, b) => score(a.moduleId) - score(b.moduleId));
  }, [modules, pillarShortcuts]);

  const orb = PILLAR_ORB[pillarId];
  const count = orderedModules.length;
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

          {orderedModules.length === 0 ? (
            <p className="rounded-2xl border border-white/60 bg-white/50 px-5 py-8 text-center text-sm text-[var(--dash-mid)] backdrop-blur">
              Aucun module accessible pour votre profil. Contactez un administrateur si besoin.
            </p>
          ) : (
            <div className={`grid min-h-0 flex-1 gap-3 sm:gap-4 ${gridClass} auto-rows-fr`}>
              {orderedModules.map((cat, i) => (
                <ModuleCard
                  key={cat.moduleId}
                  category={cat}
                  index={i}
                  orbClass={orb}
                  notifCount={notificationCountForModule(cat.moduleId, notifications)}
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
