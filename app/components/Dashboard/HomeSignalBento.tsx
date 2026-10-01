"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import {
  notificationCountForShortcut,
  type DashboardNotification,
  type DashboardShortcut,
  type DashboardShortcutSlide,
  type DashboardShortcutTone,
} from "@/app/lib/dashboard-signals";
import { MODULE_EMOJI } from "@/app/lib/pillar-module-routes";

type Face = {
  id: string;
  label: string;
  detail?: string;
  badge?: string;
  count?: number;
  href: string;
};

type HomeTile = {
  id: string;
  moduleId: string;
  tone: DashboardShortcutTone;
  emoji: string;
  /** Libellé de catégorie (stable pendant le carrousel). */
  categoryLabel: string;
  faces: Face[];
};

const MODULE_CATEGORY_LABEL: Record<string, string> = {
  travels: "Sorties scolaires",
  absences: "Absences RH",
  internat: "Internat",
  "accueil-absences": "Absences élèves",
  "absences-accueil-consultation": "Absences élèves",
  "vs-appels": "Appels",
  "vs-absences": "Absences élèves",
  "eleve-dossier": "Dossiers élèves",
  "requests-staff": "Demandes",
  "photocopies-couleur": "Photocopies",
  "prof-room": "Salles",
  stages: "Stages",
  sante: "Santé",
};

function isHomeWorthy(s: DashboardShortcut, notifCount: number): boolean {
  if (s.pillarOnly) return false;
  if (!s.rich && notifCount <= 0) return false;
  if (notifCount > 0) return true;
  if (s.tone === "warn" || s.tone === "action") return true;
  if (s.tone === "info" && (s.badge || s.detail)) return true;
  return Boolean(s.badge && s.rich);
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

function slideToFace(slide: DashboardShortcutSlide, fallbackHref: string): Face {
  return {
    id: slide.id,
    label: slide.label,
    detail: slide.detail,
    badge: slide.badge,
    count: slide.count,
    href: slide.href || fallbackHref,
  };
}

function shortcutToFaces(s: DashboardShortcut, count: number): Face[] {
  if (s.slides && s.slides.length > 0) {
    return s.slides.map((sl) =>
      slideToFace(
        {
          ...sl,
          count: sl.count ?? (sl.id === s.id ? count : sl.count),
        },
        s.href,
      ),
    );
  }
  return [
    {
      id: s.id,
      label: s.label,
      detail: s.detail,
      badge: s.badge,
      count: count > 0 ? count : undefined,
      href: s.href,
    },
  ];
}

function dedupeFaces(faces: Face[]): Face[] {
  const seen = new Set<string>();
  const out: Face[] = [];
  for (const face of faces) {
    const key = face.id || `${face.label}|${face.href}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(face);
  }
  return out;
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

function HomeSignalCard({ tile }: { tile: HomeTile }) {
  const [slideIdx, setSlideIdx] = useState(0);
  const faces = tile.faces;
  const multi = faces.length > 1;

  useEffect(() => {
    if (!multi) return;
    const id = window.setInterval(() => {
      setSlideIdx((i) => (i + 1) % faces.length);
    }, 3800);
    return () => window.clearInterval(id);
  }, [multi, faces.length]);

  const active = faces[slideIdx] ?? faces[0]!;

  return (
    <Link
      href={active.href}
      className={`group relative flex min-h-[11rem] flex-col overflow-hidden rounded-[1.75rem] p-5 transition duration-300 hover:-translate-y-0.5 hover:shadow-[0_22px_50px_-28px_rgba(0,0,0,0.45)] sm:p-5 ${tileShell(
        tile.tone,
      )}`}
    >
      <div
        className="pointer-events-none absolute -right-8 -top-8 h-28 w-28 rounded-full opacity-40 blur-2xl transition group-hover:opacity-70"
        style={{
          background:
            tile.tone === "action"
              ? "rgba(20,20,20,0.12)"
              : tile.tone === "warn"
                ? "rgba(251,191,36,0.35)"
                : "rgba(212,255,55,0.28)",
        }}
        aria-hidden
      />

      <div className="relative flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p
            className={`text-[10px] font-semibold uppercase tracking-[0.16em] ${
              tile.tone === "action"
                ? "text-[var(--dash-ink)]/55"
                : tile.tone === "warn"
                  ? "text-amber-200/70"
                  : tile.tone === "info"
                    ? "text-white/50"
                    : "text-neutral-400"
            }`}
          >
            {tile.categoryLabel}
          </p>
        </div>
        <span
          className={`inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl text-lg ${
            tile.tone === "action"
              ? "bg-black/10"
              : tile.tone === "warn"
                ? "bg-amber-400/15"
                : tile.tone === "info"
                  ? "bg-white/10"
                  : "bg-[color:var(--dash-soft-muted)]"
          }`}
          aria-hidden
        >
          {tile.emoji}
        </span>
      </div>

      <div className="relative mt-3 min-h-[5.75rem] flex-1">
        {faces.map((face, i) => {
          const visible = i === slideIdx;
          const faceCount = face.count ?? 0;
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
              {faceCount > 0 ? (
                <p className="text-4xl font-black leading-none tracking-tight sm:text-5xl">
                  {faceCount > 99 ? "99+" : faceCount}
                </p>
              ) : null}
              <div className={`flex flex-wrap items-center gap-2 ${faceCount > 0 ? "mt-2" : ""}`}>
                <p className="text-lg font-semibold tracking-tight sm:text-xl">{face.label}</p>
                {face.badge ? (
                  <span
                    className={`rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide ${
                      tile.tone === "action"
                        ? "bg-[var(--dash-ink)] text-white"
                        : tile.tone === "warn"
                          ? "bg-amber-400 text-[#1a1208]"
                          : tile.tone === "info"
                            ? "bg-[color:var(--dash-lime)] text-[var(--dash-ink)]"
                            : "bg-[color:var(--dash-soft)] text-[var(--dash-ink)]"
                    }`}
                  >
                    {face.badge}
                  </span>
                ) : null}
              </div>
              {face.detail ? (
                <p
                  className={`mt-1 line-clamp-2 text-sm leading-relaxed ${
                    tile.tone === "action"
                      ? "text-[var(--dash-ink)]/70"
                      : tile.tone === "warn"
                        ? "text-amber-100/75"
                        : tile.tone === "info"
                          ? "text-white/65"
                          : "text-neutral-500"
                  }`}
                >
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
                    ? tile.tone === "action"
                      ? "w-4 bg-[var(--dash-ink)]"
                      : tile.tone === "warn"
                        ? "w-4 bg-amber-400"
                        : tile.tone === "info"
                          ? "w-4 bg-[color:var(--dash-lime)]"
                          : "w-4 bg-[var(--dash-ink)]"
                    : tile.tone === "action"
                      ? "w-1.5 bg-[var(--dash-ink)]/25"
                      : tile.tone === "warn"
                        ? "w-1.5 bg-amber-200/35"
                        : tile.tone === "info"
                          ? "w-1.5 bg-white/30"
                          : "w-1.5 bg-neutral-300"
                }`}
              />
            ))}
          </div>
        ) : (
          <span
            className={`text-[11px] font-semibold uppercase tracking-[0.14em] ${
              tile.tone === "action"
                ? "text-[var(--dash-ink)]/55"
                : tile.tone === "warn"
                  ? "text-amber-200/70"
                  : tile.tone === "info"
                    ? "text-white/50"
                    : "text-neutral-400"
            }`}
          >
            Ouvrir
          </span>
        )}
        <span
          className={`inline-flex h-9 w-9 items-center justify-center rounded-full text-lg transition group-hover:translate-x-0.5 ${
            tile.tone === "action"
              ? "bg-[var(--dash-ink)] text-white"
              : tile.tone === "warn"
                ? "bg-amber-400 text-[#1a1208]"
                : tile.tone === "info"
                  ? "bg-[color:var(--dash-lime)] text-[var(--dash-ink)]"
                  : "bg-[var(--dash-ink)] text-white"
          }`}
          aria-hidden
        >
          →
        </span>
      </div>
    </Link>
  );
}

type Props = {
  shortcuts: DashboardShortcut[];
  notifications: DashboardNotification[];
  loading?: boolean;
};

/**
 * Bento signaux : 1 tuile par module, faces fusionnées + transition.
 */
export default function HomeSignalBento({ shortcuts, notifications, loading }: Props) {
  const tiles = useMemo(() => {
    const worthy = shortcuts
      .map((s) => ({
        shortcut: s,
        count: notificationCountForShortcut(s, notifications),
      }))
      .filter(({ shortcut, count }) => isHomeWorthy(shortcut, count));

    const byModule = new Map<string, typeof worthy>();
    for (const item of worthy) {
      const key = item.shortcut.moduleId;
      const list = byModule.get(key) ?? [];
      list.push(item);
      byModule.set(key, list);
    }

    const grouped: HomeTile[] = [];
    for (const [moduleId, items] of byModule) {
      items.sort((a, b) => {
        const tr = toneRank(a.shortcut.tone) - toneRank(b.shortcut.tone);
        if (tr !== 0) return tr;
        return b.count - a.count;
      });
      const faces = dedupeFaces(
        items.flatMap(({ shortcut, count }) => shortcutToFaces(shortcut, count)),
      );
      if (faces.length === 0) continue;
      const tone = bestTone(items.map((i) => i.shortcut.tone));
      const primary = items[0]!.shortcut;
      grouped.push({
        id: `signal-${moduleId}`,
        moduleId,
        tone,
        emoji: primary.emoji || MODULE_EMOJI[moduleId] || "•",
        categoryLabel: MODULE_CATEGORY_LABEL[moduleId] || primary.label,
        faces,
      });
    }

    grouped.sort((a, b) => {
      const tr = toneRank(a.tone) - toneRank(b.tone);
      if (tr !== 0) return tr;
      const ac = a.faces.reduce((s, f) => s + (f.count || 0), 0);
      const bc = b.faces.reduce((s, f) => s + (f.count || 0), 0);
      return bc - ac;
    });

    const coveredModules = new Set(grouped.map((t) => t.moduleId));
    const coveredFaceIds = new Set(grouped.flatMap((t) => t.faces.map((f) => f.id)));

    for (const n of notifications) {
      if (n.count <= 0) continue;
      if (coveredModules.has(n.moduleId)) continue;
      if (coveredFaceIds.has(n.id)) continue;
      grouped.push({
        id: `notif-${n.id}`,
        moduleId: n.moduleId,
        tone: "warn",
        emoji: MODULE_EMOJI[n.moduleId] || "⚡",
        categoryLabel: MODULE_CATEGORY_LABEL[n.moduleId] || n.label,
        faces: [
          {
            id: n.id,
            label: n.label,
            detail: n.detail,
            badge: "À traiter",
            count: n.count,
            href: n.href,
          },
        ],
      });
      coveredModules.add(n.moduleId);
    }

    return grouped
      .sort((a, b) => toneRank(a.tone) - toneRank(b.tone))
      .slice(0, 6);
  }, [shortcuts, notifications]);

  if (loading && tiles.length === 0) {
    return (
      <div className="space-y-3">
        <div className="px-1">
          <h2 className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--dash-mid)]">
            Signaux
          </h2>
        </div>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <div
              key={i}
              className="h-44 animate-pulse rounded-[1.75rem] bg-black/5"
              style={{ animationDelay: `${i * 80}ms` }}
            />
          ))}
        </div>
      </div>
    );
  }

  if (tiles.length === 0) {
    return (
      <div className="space-y-3">
        <div className="px-1">
          <h2 className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--dash-mid)]">
            Signaux
          </h2>
        </div>
        <div className="rounded-[1.75rem] border border-dashed border-black/10 bg-white/60 px-6 py-8 text-center">
          <p className="text-sm font-semibold text-[var(--dash-ink)]">Rien d’urgent pour vous</p>
          <p className="mt-1 text-sm text-neutral-500">
            Les alertes n’apparaissent ici que s’elles vous concernent.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="px-1">
        <h2 className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--dash-mid)]">
          Signaux
        </h2>
      </div>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {tiles.map((tile) => (
          <HomeSignalCard key={tile.id} tile={tile} />
        ))}
      </div>
    </div>
  );
}
