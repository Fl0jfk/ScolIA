"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";
import HomeSignalBento from "@/app/components/Dashboard/HomeSignalBento";
import {
  DASHBOARD_PILLARS,
  categoriesForPillar,
  type DashboardPillarId,
} from "@/app/lib/dashboard-pillars";
import type { DashboardCategory } from "@/app/lib/intranet-modules";
import {
  notificationCountForModule,
  type DashboardNotification,
  type DashboardShortcut,
  type DashboardShortcutTone,
} from "@/app/lib/dashboard-signals";
import { useDashboardSignals } from "@/app/hooks/useDashboardSignals";
import { MODULE_EMOJI, moduleHref } from "@/app/lib/pillar-module-routes";
import ScoliaCompactBar from "@/app/components/scolia/ScoliaCompactBar";

type Props = {
  pillarId: DashboardPillarId;
  categories: DashboardCategory[];
  accessibleModuleIds: Set<string>;
  roles?: string[];
  orgAdmin?: boolean;
};

type QuickAction = {
  href: string;
  label: string;
  primary?: boolean;
};

type ModuleFace = {
  id: string;
  label: string;
  detail?: string;
  badge?: string;
  count?: number;
  href: string;
};

type ModuleTile = {
  id: string;
  moduleId: string;
  title: string;
  emoji: string;
  href: string;
  tone: DashboardShortcutTone;
  faces: ModuleFace[];
  actions: QuickAction[];
  notifCount: number;
};

const PILLAR_EMOJI: Record<DashboardPillarId, string> = {
  administratif: "🗂️",
  etablissement: "🏫",
  services: "🛠️",
  vie_scolaire: "📋",
  compta_rh: "💼",
  sante: "🩺",
};

function hubNotificationCount(moduleId: string, notifications: DashboardNotification[]): number {
  if (moduleId === "rh") {
    return (
      notificationCountForModule("rh", notifications) +
      notificationCountForModule("absences", notifications) +
      notificationCountForModule("demandes-hse", notifications)
    );
  }
  return notificationCountForModule(moduleId, notifications);
}

function toneRank(tone: DashboardShortcutTone | undefined): number {
  if (tone === "warn") return 0;
  if (tone === "action") return 1;
  if (tone === "info") return 2;
  return 3;
}

function bestTone(tones: Array<DashboardShortcutTone | undefined>): DashboardShortcutTone {
  return [...tones].sort((a, b) => toneRank(a) - toneRank(b))[0] || "neutral";
}

function tileShell(tone: DashboardShortcutTone): string {
  if (tone === "warn") {
    return "bg-[#1a1208] text-amber-50 ring-1 ring-amber-400/30";
  }
  if (tone === "action") {
    return "bg-[color:var(--dash-lime)] text-[var(--dash-ink)] ring-1 ring-black/5";
  }
  if (tone === "info") {
    return "bg-[var(--dash-ink)] text-white ring-1 ring-white/10";
  }
  return "bg-white text-[var(--dash-ink)] ring-1 ring-black/6";
}

function softLabelClass(tone: DashboardShortcutTone): string {
  if (tone === "action") return "text-[var(--dash-ink)]/55";
  if (tone === "warn") return "text-amber-200/70";
  if (tone === "info") return "text-white/50";
  return "text-neutral-400";
}

function detailClass(tone: DashboardShortcutTone): string {
  if (tone === "action") return "text-[var(--dash-ink)]/70";
  if (tone === "warn") return "text-amber-100/75";
  if (tone === "info") return "text-white/65";
  return "text-neutral-500";
}

function emojiShell(tone: DashboardShortcutTone): string {
  if (tone === "action") return "bg-black/10";
  if (tone === "warn") return "bg-amber-400/15";
  if (tone === "info") return "bg-white/10";
  return "bg-[color:var(--dash-soft-muted)]";
}

function badgeShell(tone: DashboardShortcutTone): string {
  if (tone === "action") return "bg-[var(--dash-ink)] text-white";
  if (tone === "warn") return "bg-amber-400 text-[#1a1208]";
  if (tone === "info") return "bg-[color:var(--dash-lime)] text-[var(--dash-ink)]";
  return "bg-[color:var(--dash-soft)] text-[var(--dash-ink)]";
}

function arrowShell(tone: DashboardShortcutTone): string {
  if (tone === "action") return "bg-[var(--dash-ink)] text-white";
  if (tone === "warn") return "bg-amber-400 text-[#1a1208]";
  if (tone === "info") return "bg-[color:var(--dash-lime)] text-[var(--dash-ink)]";
  return "bg-[var(--dash-ink)] text-white";
}

function actionChipClass(tone: DashboardShortcutTone, primary?: boolean): string {
  if (primary) {
    if (tone === "action") return "bg-[var(--dash-ink)] text-white";
    if (tone === "warn") return "bg-amber-400 text-[#1a1208]";
    if (tone === "info") return "bg-[color:var(--dash-lime)] text-[var(--dash-ink)]";
    return "bg-[var(--dash-ink)] text-white";
  }
  if (tone === "action") return "bg-black/10 text-[var(--dash-ink)]";
  if (tone === "warn") return "bg-amber-400/15 text-amber-50";
  if (tone === "info") return "bg-white/10 text-white";
  return "bg-[color:var(--dash-soft-muted)] text-[var(--dash-ink)]";
}

function quickActionsForModule(
  moduleId: string,
  accessibleModuleIds?: Set<string>,
): QuickAction[] {
  if (moduleId === "travels") {
    return [
      { href: "/travels/simple", label: "+ Sortie simple", primary: true },
      { href: "/travels/complex", label: "+ Voyage / bus" },
    ];
  }
  if (moduleId === "internat") {
    return [{ href: "/gestion-internat", label: "Ouvrir l’appel", primary: true }];
  }
  if (moduleId === "stages") {
    return [{ href: "/stages", label: "Voir les conventions" }];
  }
  if (moduleId === "requests-staff") {
    return [
      { href: "/faire-une-demande", label: "+ Nouvelle demande", primary: true },
      { href: "/requests", label: "File" },
    ];
  }
  if (moduleId === "prof-room") {
    return [{ href: "/prof-room", label: "Réserver une salle", primary: true }];
  }
  if (moduleId === "office") {
    return [
      { href: "/documents/office", label: "Ouvrir", primary: true },
      { href: "/documents/writer", label: "Texte" },
      { href: "/documents/calc", label: "Tableur" },
      { href: "/documents/impress", label: "Diapo" },
    ];
  }
  if (moduleId === "photocopies-couleur") {
    return [
      { href: "/photocopies", label: "Ouvrir" },
      { href: "/photocopies#file-impression", label: "File impression" },
    ];
  }
  if (moduleId === "accueil-absences") {
    const canConsult = accessibleModuleIds?.has("absences-accueil-consultation");
    const canAppels = accessibleModuleIds?.has("vs-appels");
    const actions: QuickAction[] = [
      { href: "/vie-scolaire/absences?tab=declarer", label: "Déclarer", primary: true },
    ];
    if (canConsult) {
      actions.push({ href: "/vie-scolaire/absences?tab=consulter", label: "Consulter" });
    }
    if (canAppels) {
      actions.push({ href: "/vie-scolaire/absences?tab=appels", label: "Appels" });
    }
    return actions;
  }
  return [];
}

function facesForModule(
  moduleId: string,
  shortcuts: DashboardShortcut[],
  fallbackHref: string,
  fallbackDetail: string,
  notifCount: number,
): ModuleFace[] {
  const relatedIds =
    moduleId === "rh"
      ? new Set(["rh", "absences", "demandes-hse"])
      : moduleId === "accueil-absences"
        ? new Set(["accueil-absences", "absences-accueil-consultation"])
        : new Set([moduleId]);

  const related = shortcuts.filter((s) => relatedIds.has(s.moduleId));
  const faces: ModuleFace[] = [];
  const seen = new Set<string>();

  for (const s of related) {
    if (s.slides?.length) {
      for (const slide of s.slides) {
        const key = slide.id || `${slide.label}|${slide.href || ""}`;
        if (seen.has(key)) continue;
        seen.add(key);
        faces.push({
          id: key,
          label: slide.label,
          detail: slide.detail,
          badge: slide.badge,
          count: slide.count,
          href: slide.href || s.href || fallbackHref,
        });
      }
      continue;
    }

    const actionable =
      s.pillarOnly ||
      s.rich ||
      Boolean(s.detail) ||
      Boolean(s.badge) ||
      s.tone === "warn" ||
      s.tone === "action";
    if (!actionable) continue;
    const key = s.id || `${s.label}|${s.href}`;
    if (seen.has(key)) continue;
    seen.add(key);
    faces.push({
      id: key,
      label: s.label,
      detail: s.detail,
      badge: s.badge,
      href: s.href || fallbackHref,
    });
  }

  if (faces.length === 0) {
    faces.push({
      id: `${moduleId}-default`,
      label: fallbackDetail || "Accéder au module",
      count: notifCount > 0 ? notifCount : undefined,
      href: fallbackHref,
    });
  } else if (notifCount > 0 && !faces.some((f) => (f.count || 0) > 0)) {
    faces[0] = { ...faces[0]!, count: notifCount };
  }

  return faces.slice(0, 5);
}

function ModuleBentoCard({ tile }: { tile: ModuleTile }) {
  const [slideIdx, setSlideIdx] = useState(0);
  const faces = tile.faces;
  const multi = faces.length > 1;
  const tone = tile.tone;

  useEffect(() => {
    if (!multi) return;
    const id = window.setInterval(() => {
      setSlideIdx((i) => (i + 1) % faces.length);
    }, 3800);
    return () => window.clearInterval(id);
  }, [multi, faces.length]);

  const active = faces[slideIdx] ?? faces[0]!;
  const faceCount = active.count ?? (slideIdx === 0 ? tile.notifCount : 0);
  const cardHref = active.href || tile.href;

  return (
    <div
      className={`group relative flex min-h-[12.5rem] flex-col overflow-hidden rounded-[1.75rem] transition duration-300 hover:-translate-y-0.5 hover:shadow-[0_22px_50px_-28px_rgba(0,0,0,0.45)] ${tileShell(
        tone,
      )}`}
    >
      <div
        className="pointer-events-none absolute -right-8 -top-8 h-28 w-28 rounded-full opacity-40 blur-2xl transition group-hover:opacity-70"
        style={{
          background:
            tone === "action"
              ? "rgba(20,20,20,0.12)"
              : tone === "warn"
                ? "rgba(251,191,36,0.35)"
                : "rgba(212,255,55,0.28)",
        }}
        aria-hidden
      />

      <Link
        href={cardHref}
        className="relative flex min-h-0 flex-1 flex-col p-5 outline-none sm:p-5"
        aria-label={`Ouvrir ${tile.title}`}
      >
        <div className="relative flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p className={`text-[10px] font-semibold uppercase tracking-[0.16em] ${softLabelClass(tone)}`}>
              Module
            </p>
            <h2 className="mt-1 truncate text-lg font-semibold tracking-tight sm:text-xl">{tile.title}</h2>
          </div>
          <span
            className={`inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl text-xl ${emojiShell(tone)}`}
            aria-hidden
          >
            {tile.emoji}
          </span>
        </div>

        <div className="relative mt-3 min-h-[4.75rem] flex-1">
          {faces.map((face, i) => {
            const visible = i === slideIdx;
            const count = face.count ?? (i === 0 ? tile.notifCount : 0);
            return (
              <div
                key={face.id}
                className={`absolute inset-0 flex flex-col justify-center transition-all duration-500 ease-out ${
                  visible
                    ? "translate-y-0 opacity-100"
                    : "pointer-events-none translate-y-2 opacity-0"
                }`}
                aria-hidden={!visible}
              >
                {count > 0 ? (
                  <p className="text-4xl font-black leading-none tracking-tight sm:text-5xl">
                    {count > 99 ? "99+" : count}
                  </p>
                ) : null}
                <div className={`flex flex-wrap items-center gap-2 ${count > 0 ? "mt-2" : ""}`}>
                  <p className="text-base font-semibold tracking-tight sm:text-lg">{face.label}</p>
                  {face.badge ? (
                    <span
                      className={`rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ${badgeShell(
                        tone,
                      )}`}
                    >
                      {face.badge}
                    </span>
                  ) : null}
                </div>
                {face.detail ? (
                  <p className={`mt-1 line-clamp-2 text-sm leading-relaxed ${detailClass(tone)}`}>
                    {face.detail}
                  </p>
                ) : null}
              </div>
            );
          })}
        </div>

        <div className="relative mt-3 flex items-center justify-between gap-2">
          {multi ? (
            <div className="flex items-center gap-1.5" aria-hidden>
              {faces.map((f, i) => (
                <span
                  key={f.id}
                  className={`h-1.5 rounded-full transition-all duration-300 ${
                    i === slideIdx
                      ? tone === "warn"
                        ? "w-4 bg-amber-400"
                        : tone === "info"
                          ? "w-4 bg-[color:var(--dash-lime)]"
                          : "w-4 bg-[var(--dash-ink)]"
                      : tone === "action"
                        ? "w-1.5 bg-[var(--dash-ink)]/25"
                        : tone === "warn"
                          ? "w-1.5 bg-amber-200/35"
                          : tone === "info"
                            ? "w-1.5 bg-white/30"
                            : "w-1.5 bg-neutral-300"
                  }`}
                />
              ))}
            </div>
          ) : (
            <span className={`text-[11px] font-semibold uppercase tracking-[0.14em] ${softLabelClass(tone)}`}>
              {faceCount > 0 ? "À traiter" : "Ouvrir"}
            </span>
          )}
          <span
            className={`inline-flex h-9 w-9 items-center justify-center rounded-full text-lg transition group-hover:translate-x-0.5 ${arrowShell(
              tone,
            )}`}
            aria-hidden
          >
            →
          </span>
        </div>
      </Link>

      {tile.actions.length > 0 ? (
        <div className="relative flex flex-wrap gap-1.5 px-5 pb-5">
          {tile.actions.map((action) => (
            <Link
              key={`${action.href}-${action.label}`}
              href={action.href}
              className={`rounded-full px-3 py-1.5 text-[11px] font-semibold transition hover:opacity-90 ${actionChipClass(
                tone,
                action.primary,
              )}`}
            >
              {action.label}
            </Link>
          ))}
        </div>
      ) : null}
    </div>
  );
}

export default function PillarModuleDashboard({
  pillarId,
  categories,
  accessibleModuleIds,
  roles = [],
  orgAdmin = false,
}: Props) {
  const pillar = DASHBOARD_PILLARS.find((p) => p.id === pillarId)!;
  const { shortcuts, notifications, loading: loadingSignals } = useDashboardSignals({
    pollIntervalMs: 0,
  });

  const modules = useMemo(() => {
    return categoriesForPillar(pillar, categories, roles, { orgAdmin }).filter((c) =>
      accessibleModuleIds.has(c.moduleId),
    );
  }, [pillar, categories, accessibleModuleIds, roles, orgAdmin]);

  const pillarShortcuts = useMemo(
    () => shortcuts.filter((s) => s.pillarId === pillarId),
    [shortcuts, pillarId],
  );

  const pillarNotifications = useMemo(() => {
    const moduleIds = new Set(modules.map((m) => m.moduleId));
    if (moduleIds.has("rh")) {
      moduleIds.add("absences");
      moduleIds.add("demandes-hse");
    }
    if (moduleIds.has("accueil-absences")) {
      moduleIds.add("absences-accueil-consultation");
    }
    return notifications.filter((n) => moduleIds.has(n.moduleId));
  }, [notifications, modules]);

  const tiles = useMemo((): ModuleTile[] => {
    const built: ModuleTile[] = modules.map((category) => {
      const canDeclare = accessibleModuleIds.has("accueil-absences");
      const canConsult = accessibleModuleIds.has("absences-accueil-consultation");
      const isAbsencesHub = category.moduleId === "accueil-absences";
      const title =
        isAbsencesHub && canDeclare && !canConsult
          ? "Absence déclarée à l'accueil"
          : isAbsencesHub && !canDeclare && canConsult
            ? "Absences déclarées à l'accueil"
            : category.name;
      const href =
        isAbsencesHub && canDeclare && !canConsult
          ? "/vie-scolaire/absences?tab=declarer"
          : isAbsencesHub && !canDeclare && canConsult
            ? "/vie-scolaire/absences?tab=consulter"
            : category.link || moduleHref(category.moduleId);
      const related = pillarShortcuts.filter((s) => {
        if (category.moduleId === "rh") {
          return s.moduleId === "rh" || s.moduleId === "absences" || s.moduleId === "demandes-hse";
        }
        if (category.moduleId === "accueil-absences") {
          return (
            s.moduleId === "accueil-absences" || s.moduleId === "absences-accueil-consultation"
          );
        }
        return s.moduleId === category.moduleId;
      });
      const notifCount = hubNotificationCount(category.moduleId, notifications);
      const tone =
        notifCount > 0
          ? "warn"
          : bestTone(related.map((s) => s.tone));
      return {
        id: category.moduleId,
        moduleId: category.moduleId,
        title,
        emoji: MODULE_EMOJI[category.moduleId] || "›",
        href,
        tone,
        faces: facesForModule(
          category.moduleId,
          pillarShortcuts,
          href,
          category.description || pillar.description,
          notifCount,
        ),
        actions: quickActionsForModule(category.moduleId, accessibleModuleIds),
        notifCount,
      };
    });

    return built.sort((a, b) => {
      const tr = toneRank(a.tone) - toneRank(b.tone);
      if (tr !== 0) return tr;
      return b.notifCount - a.notifCount;
    });
  }, [modules, pillarShortcuts, notifications, accessibleModuleIds, pillar.description]);

  return (
    <DashboardThemeRoot>
      <div className="relative min-h-[calc(100dvh-1rem)] overflow-x-hidden bg-[color:var(--dash-surface)]">
        <div className="pointer-events-none absolute inset-0" aria-hidden>
          <div className="absolute -left-24 top-0 h-[26rem] w-[26rem] rounded-full bg-[color:var(--dash-lime)]/25 blur-3xl" />
          <div className="absolute right-0 top-24 h-[20rem] w-[20rem] rounded-full bg-[color:var(--dash-soft)]/80 blur-3xl" />
        </div>

        <main
          className="relative mx-auto flex w-full max-w-[90rem] flex-col gap-5 px-3 py-4 sm:gap-6 sm:px-5 lg:px-7 lg:py-5"
          aria-label={pillar.title}
        >
          <header className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
            <div className="min-w-0 px-1">
              <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--dash-mid)]">
                Espace
              </p>
              <div className="mt-1 flex items-center gap-2.5">
                <span className="text-2xl leading-none" aria-hidden>
                  {PILLAR_EMOJI[pillarId]}
                </span>
                <h1 className="truncate text-2xl font-semibold tracking-tight text-[var(--dash-ink)] sm:text-3xl">
                  {pillar.title}
                </h1>
              </div>
              <p className="mt-1 max-w-2xl text-sm text-neutral-500">{pillar.description}</p>
            </div>
            <ScoliaCompactBar className="w-full sm:max-w-md" />
          </header>

          <HomeSignalBento
            shortcuts={pillarShortcuts}
            notifications={pillarNotifications}
            loading={loadingSignals}
          />

          <section className="space-y-3">
            <div className="px-1">
              <h2 className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--dash-mid)]">
                Modules
              </h2>
            </div>

            {tiles.length === 0 ? (
              <div className="rounded-[1.75rem] border border-dashed border-black/10 bg-white/60 px-6 py-8 text-center">
                <p className="text-sm font-semibold text-[var(--dash-ink)]">
                  Aucun module accessible
                </p>
                <p className="mt-1 text-sm text-neutral-500">
                  Contactez un administrateur si vous pensez devoir y accéder.
                </p>
              </div>
            ) : (
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {tiles.map((tile) => (
                  <ModuleBentoCard key={tile.id} tile={tile} />
                ))}
              </div>
            )}
          </section>
        </main>
      </div>
    </DashboardThemeRoot>
  );
}
