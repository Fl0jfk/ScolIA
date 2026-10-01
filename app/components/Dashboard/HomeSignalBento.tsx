"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import {
  notificationCountForShortcut,
  type DashboardNotification,
  type DashboardShortcut,
  type DashboardShortcutTone,
} from "@/app/lib/dashboard-signals";
import { MODULE_EMOJI } from "@/app/lib/pillar-module-routes";

type HomeTile = {
  id: string;
  href: string;
  label: string;
  detail?: string;
  badge?: string;
  tone: DashboardShortcutTone;
  emoji: string;
  count: number;
  moduleId: string;
  slides?: DashboardShortcut["slides"];
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

function tileShell(tone: DashboardShortcutTone, index: number): string {
  const span =
    index === 0
      ? "sm:col-span-2 sm:row-span-2 min-h-[14rem]"
      : index === 1
        ? "sm:col-span-1 min-h-[10.5rem]"
        : index === 2
          ? "sm:col-span-1 min-h-[10.5rem]"
          : "min-h-[9.5rem]";

  if (tone === "warn") {
    return `${span} bg-[#1a1208] text-amber-50 ring-1 ring-amber-400/30`;
  }
  if (tone === "action") {
    return `${span} bg-[color:var(--dash-lime)] text-[var(--dash-ink)] ring-1 ring-black/5`;
  }
  if (tone === "info") {
    return `${span} bg-[var(--dash-ink)] text-white ring-1 ring-white/10`;
  }
  return `${span} bg-white text-[var(--dash-ink)] ring-1 ring-black/6`;
}

function HomeSignalCard({ tile, index }: { tile: HomeTile; index: number }) {
  const [slideIdx, setSlideIdx] = useState(0);
  const slides = tile.slides?.length ? tile.slides : null;

  useEffect(() => {
    if (!slides || slides.length < 2) return;
    const id = window.setInterval(() => {
      setSlideIdx((i) => (i + 1) % slides.length);
    }, 3200);
    return () => window.clearInterval(id);
  }, [slides]);

  const active = slides?.[slideIdx];
  const href = active?.href || tile.href;
  const label = active?.label || tile.label;
  const detail = active?.detail || tile.detail;
  const badge = active?.badge || tile.badge;
  const count = active?.count ?? tile.count;
  const showBigCount = count > 0 || tile.tone === "warn" || tile.tone === "action";

  return (
    <Link
      href={href}
      className={`group relative flex flex-col justify-between overflow-hidden rounded-[1.75rem] p-5 transition duration-300 hover:-translate-y-0.5 hover:shadow-[0_22px_50px_-28px_rgba(0,0,0,0.45)] sm:p-6 ${tileShell(
        tile.tone,
        index,
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
        <span
          className={`inline-flex h-11 w-11 items-center justify-center rounded-2xl text-xl ${
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
        {badge ? (
          <span
            className={`rounded-full px-2.5 py-1 text-[10px] font-bold uppercase tracking-wide ${
              tile.tone === "action"
                ? "bg-[var(--dash-ink)] text-white"
                : tile.tone === "warn"
                  ? "bg-amber-400 text-[#1a1208]"
                  : tile.tone === "info"
                    ? "bg-[color:var(--dash-lime)] text-[var(--dash-ink)]"
                    : "bg-[color:var(--dash-soft)] text-[var(--dash-ink)]"
            }`}
          >
            {badge}
          </span>
        ) : null}
      </div>

      <div className="relative mt-6 space-y-2">
        {showBigCount && count > 0 ? (
          <p
            className={`font-black leading-none tracking-tight ${
              index === 0 ? "text-6xl sm:text-7xl" : "text-4xl sm:text-5xl"
            }`}
          >
            {count > 99 ? "99+" : count}
          </p>
        ) : null}
        <p
          className={`font-semibold tracking-tight ${
            index === 0 ? "text-2xl sm:text-3xl" : "text-lg sm:text-xl"
          }`}
        >
          {label}
        </p>
        {detail ? (
          <p
            className={`line-clamp-2 text-sm leading-relaxed ${
              tile.tone === "action"
                ? "text-[var(--dash-ink)]/70"
                : tile.tone === "warn"
                  ? "text-amber-100/75"
                  : tile.tone === "info"
                    ? "text-white/65"
                    : "text-neutral-500"
            }`}
          >
            {detail}
          </p>
        ) : null}
      </div>

      <div className="relative mt-5 flex items-center justify-between gap-2">
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
 * Bento d’accueil : uniquement les signaux forts (pas la grille modules).
 */
export default function HomeSignalBento({ shortcuts, notifications, loading }: Props) {
  const tiles = useMemo(() => {
    const fromShortcuts: HomeTile[] = shortcuts
      .map((s) => {
        const count = notificationCountForShortcut(s, notifications);
        return {
          shortcut: s,
          count,
        };
      })
      .filter(({ shortcut, count }) => isHomeWorthy(shortcut, count))
      .sort((a, b) => {
        const tr = toneRank(a.shortcut.tone) - toneRank(b.shortcut.tone);
        if (tr !== 0) return tr;
        return b.count - a.count;
      })
      .slice(0, 8)
      .map(({ shortcut: s, count }) => ({
        id: s.id,
        href: s.href,
        label: s.label,
        detail: s.detail,
        badge: s.badge,
        tone: (s.tone || "neutral") as DashboardShortcutTone,
        emoji: s.emoji || MODULE_EMOJI[s.moduleId] || "•",
        count,
        moduleId: s.moduleId,
        slides: s.slides,
      }));

    const covered = new Set(fromShortcuts.map((t) => t.id));
    const orphanNotifs: HomeTile[] = notifications
      .filter((n) => n.count > 0 && !covered.has(n.id))
      .slice(0, 4)
      .map((n) => ({
        id: n.id,
        href: n.href,
        label: n.label,
        detail: n.detail,
        badge: "À traiter",
        tone: "warn" as const,
        emoji: MODULE_EMOJI[n.moduleId] || "⚡",
        count: n.count,
        moduleId: n.moduleId,
      }));

    return [...fromShortcuts, ...orphanNotifs]
      .sort((a, b) => toneRank(a.tone) - toneRank(b.tone) || b.count - a.count)
      .slice(0, 8);
  }, [shortcuts, notifications]);

  if (loading && tiles.length === 0) {
    return (
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <div
            key={i}
            className="h-40 animate-pulse rounded-[1.75rem] bg-black/5"
            style={{ animationDelay: `${i * 80}ms` }}
          />
        ))}
      </div>
    );
  }

  if (tiles.length === 0) {
    return (
      <div className="rounded-[1.75rem] border border-dashed border-black/10 bg-white/60 px-6 py-10 text-center">
        <p className="text-sm font-semibold text-[var(--dash-ink)]">Rien d’urgent pour vous</p>
        <p className="mt-1 text-sm text-neutral-500">
          Les signaux n’apparaissent ici que s’ils vous concernent. Utilisez ScolIA pour le reste.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="flex items-end justify-between gap-3 px-1">
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--dash-mid)]">
            Signaux
          </p>
          <h2 className="text-lg font-semibold tracking-tight text-[var(--dash-ink)]">
            Ce qui compte maintenant
          </h2>
        </div>
        <p className="text-xs font-medium text-neutral-400">{tiles.length} actif{tiles.length > 1 ? "s" : ""}</p>
      </div>
      <div className="grid auto-rows-fr grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {tiles.map((tile, index) => (
          <HomeSignalCard key={tile.id} tile={tile} index={index} />
        ))}
      </div>
    </div>
  );
}
