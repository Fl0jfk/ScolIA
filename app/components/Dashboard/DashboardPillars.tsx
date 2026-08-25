"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import type { DashboardCategory } from "@/app/lib/intranet-modules";
import {
  DASHBOARD_PILLARS,
  moduleIdsForPillarViewer,
  pillarHasVisibleModules,
  type DashboardPillarDef,
  type DashboardPillarId,
} from "@/app/lib/dashboard-pillars";
import type {
  DashboardNotification,
  DashboardShortcut,
  DashboardShortcutSlide,
} from "@/app/lib/dashboard-signals";
import { notificationCountForShortcut } from "@/app/lib/dashboard-signals";
import { MODULE_EMOJI } from "@/app/lib/pillar-module-routes";
import GlassLayer from "@/app/components/GlassLayer";
import NotificationCountBadge from "@/app/components/Dashboard/NotificationCountBadge";

type Props = {
  categories: DashboardCategory[];
  shortcuts: DashboardShortcut[];
  notifications?: DashboardNotification[];
  pulseKey?: string;
  roles?: string[];
  orgAdmin?: boolean;
};

const PILLAR_ORB: Record<DashboardPillarId, string> = {
  administratif: "bg-sky-400/30",
  etablissement: "bg-cyan-400/30",
  services: "bg-teal-400/30",
  vie_scolaire: "bg-emerald-400/30",
  compta_rh: "bg-amber-400/30",
  sante: "bg-rose-400/30",
};

const PILLAR_EMOJI: Record<DashboardPillarId, string> = {
  administratif: "🗂️",
  etablissement: "🏫",
  services: "🛠️",
  vie_scolaire: "📋",
  compta_rh: "💼",
  sante: "🩺",
};

/** Toujours prioritaires sur le pilier Établissement. */
const PINNED_ADMIN_MODULE_IDS = new Set([
  "admin-settings",
  "organigramme",
  "communication",
  "evenements",
]);

function slideTextColor(hex: string): string {
  const h = hex.replace("#", "");
  if (h.length !== 6) return "#ffffff";
  const r = parseInt(h.slice(0, 2), 16) / 255;
  const g = parseInt(h.slice(2, 4), 16) / 255;
  const b = parseInt(h.slice(4, 6), 16) / 255;
  const lin = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  const lum = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
  return lum > 0.45 ? "#0f172a" : "#ffffff";
}

function ShortcutSlidesCarousel({
  slides,
  href,
  highlight,
  title,
  chip = "En cours",
  emoji = "🚪",
  fallbackColor = "#475569",
  notifCount = 0,
}: {
  slides: DashboardShortcutSlide[];
  href: string;
  highlight?: boolean;
  title: string;
  chip?: string;
  emoji?: string;
  fallbackColor?: string;
  notifCount?: number;
}) {
  const [index, setIndex] = useState(0);

  const slideKey = slides.map((s) => s.id).join("|");

  useEffect(() => {
    setIndex(0);
  }, [slideKey]);

  useEffect(() => {
    if (slides.length <= 1) return;
    const id = window.setInterval(() => {
      setIndex((i) => (i + 1) % slides.length);
    }, 3000);
    return () => window.clearInterval(id);
  }, [slides.length, slideKey]);

  const slide = slides[index] ?? slides[0]!;
  const bg = slide.colorHex || fallbackColor;
  const fg = slideTextColor(bg);
  const linkHref = slide.href || href;

  return (
    <motion.div
      layout
      className="col-span-2"
      initial={highlight ? { scale: 0.97, opacity: 0.45 } : false}
      animate={{ scale: 1, opacity: 1 }}
      transition={{ type: "spring", stiffness: 380, damping: 28 }}
    >
      <Link
        href={linkHref}
        className="group relative block cursor-pointer overflow-hidden rounded-xl border border-white/70 shadow-[0_10px_30px_-22px_rgba(15,23,42,0.45)] transition hover:-translate-y-0.5"
      >
        {highlight ? (
          <motion.span
            className="pointer-events-none absolute inset-0 z-10 bg-white/25"
            initial={{ opacity: 0.8 }}
            animate={{ opacity: 0 }}
            transition={{ duration: 1.4 }}
          />
        ) : null}
        <div className="relative min-h-[3.25rem] overflow-hidden rounded-xl px-2.5 py-2" style={{ backgroundColor: bg, color: fg }}>
          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={slide.id}
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.28 }}
              className="flex items-center gap-2"
            >
              <span
                className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-sm leading-none"
                style={{ backgroundColor: "rgba(255,255,255,0.2)" }}
                aria-hidden
              >
                {emoji}
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-1.5">
                  <p className="truncate text-[12px] font-semibold tracking-tight">{title}</p>
                  <NotificationCountBadge count={notifCount} />
                  <span
                    className="shrink-0 rounded-full px-1.5 py-0.5 text-[8px] font-bold uppercase tracking-wide"
                    style={{ backgroundColor: "rgba(255,255,255,0.22)" }}
                  >
                    {chip}
                  </span>
                </div>
                <p className="mt-0.5 truncate text-[10px] font-semibold leading-snug opacity-90">
                  {[slide.label, slide.detail].filter(Boolean).join(" · ")}
                </p>
              </div>
              {slide.badge ? (
                <span
                  className="relative shrink-0 rounded-full px-2 py-0.5 text-[9px] font-bold"
                  style={{ backgroundColor: "rgba(255,255,255,0.22)" }}
                >
                  {slide.badge}
                </span>
              ) : null}
            </motion.div>
          </AnimatePresence>
          {slides.length > 1 ? (
            <div className="mt-1.5 flex items-center justify-center gap-1">
              {slides.map((s, i) => (
                <span
                  key={s.id}
                  className="h-1 rounded-full transition-all"
                  style={{
                    width: i === index ? 10 : 4,
                    backgroundColor: fg,
                    opacity: i === index ? 0.95 : 0.35,
                  }}
                />
              ))}
            </div>
          ) : null}
        </div>
      </Link>
    </motion.div>
  );
}

function ShortcutTile({
  item,
  highlight,
  fullWidth,
  notifCount = 0,
}: {
  item: DashboardShortcut;
  highlight?: boolean;
  fullWidth?: boolean;
  notifCount?: number;
}) {
  if (item.slides && item.slides.length > 0) {
    const isTravels = item.moduleId === "travels";
    return (
      <ShortcutSlidesCarousel
        slides={item.slides}
        href={item.href}
        highlight={highlight}
        title={item.label}
        chip={isTravels ? "Aujourd'hui" : "En cours"}
        emoji={item.emoji || MODULE_EMOJI[item.moduleId] || (isTravels ? "🚌" : "🚪")}
        fallbackColor={isTravels ? "#0284c7" : "#475569"}
        notifCount={notifCount}
      />
    );
  }

  const emoji = item.emoji || MODULE_EMOJI[item.moduleId] || "›";

  return (
    <motion.div
      layout
      className={fullWidth ? "col-span-2" : "col-span-1"}
      initial={highlight ? { scale: 0.97, opacity: 0.45 } : false}
      animate={{ scale: 1, opacity: 1 }}
      transition={{ type: "spring", stiffness: 380, damping: 28 }}
    >
      <Link
        href={item.href}
        className={`group relative flex cursor-pointer items-center gap-2 rounded-xl border transition hover:-translate-y-0.5 ${
          item.rich
            ? "border-white/70 bg-white/80 px-2.5 py-2.5 shadow-[0_10px_30px_-22px_rgba(15,23,42,0.45)] hover:bg-white"
            : "border-transparent bg-white/40 px-2 py-2 hover:border-white/70 hover:bg-white/70"
        }`}
      >
        {highlight ? (
          <motion.span
            className="pointer-events-none absolute inset-0 bg-[color:var(--dash-bright)]/15"
            initial={{ opacity: 0.8 }}
            animate={{ opacity: 0 }}
            transition={{ duration: 1.4 }}
          />
        ) : null}
        <span className="relative flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-[color:var(--dash-soft-muted)]/90 ring-1 ring-[color:var(--dash-border)]/60">
          <span className="text-sm leading-none" aria-hidden>
            {emoji}
          </span>
        </span>
        <div className="relative min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-1.5">
            <p className="truncate text-[12px] font-semibold tracking-tight text-[var(--dash-ink)]">
              {item.label}
            </p>
            <NotificationCountBadge count={notifCount} />
          </div>
          {item.detail ? (
            <p className="mt-0.5 line-clamp-1 text-[10px] leading-snug text-[var(--dash-mid)]">
              {item.detail}
            </p>
          ) : null}
        </div>
        {item.badge ? (
          <span className="relative shrink-0 rounded-full bg-[var(--dash-primary)] px-2 py-0.5 text-[9px] font-bold text-white">
            {item.badge}
          </span>
        ) : null}
      </Link>
    </motion.div>
  );
}

function PillarCard({
  pillar,
  shortcuts,
  notifications,
  index,
  pulseKey,
}: {
  pillar: DashboardPillarDef;
  shortcuts: DashboardShortcut[];
  notifications: DashboardNotification[];
  index: number;
  pulseKey?: string;
}) {
  const toneRank = (s: DashboardShortcut) => {
    if (s.tone === "warn") return 0;
    if (s.tone === "action") return 1;
    if (s.tone === "info") return 2;
    return 3;
  };

  const rich = shortcuts
    .filter((s) => s.rich && s.tone !== "neutral")
    .sort((a, b) => toneRank(a) - toneRank(b));
  const plain = shortcuts.filter((s) => !s.rich || s.tone === "neutral");

  const pinned = plain.filter((s) => PINNED_ADMIN_MODULE_IDS.has(s.moduleId));
  const unpinned = plain.filter((s) => !PINNED_ADMIN_MODULE_IDS.has(s.moduleId));
  // Budget : garder les raccourcis établissement, puis le reste.
  const plainBudget = Math.max(pinned.length, Math.max(0, 8 - rich.length * 2));
  const visiblePlain = [...pinned, ...unpinned].slice(0, plainBudget);
  const visible = [...rich, ...visiblePlain];

  return (
    <motion.article
      layout
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, delay: 0.06 * index, ease: [0.22, 1, 0.36, 1] }}
      className="group relative flex min-h-0 flex-col overflow-hidden rounded-[1.5rem] border border-white/55 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)]"
    >
      <GlassLayer className="bg-white/45 backdrop-blur-2xl" />
      <div
        className={`pointer-events-none absolute -right-8 -top-10 z-0 h-36 w-36 rounded-full blur-3xl transition duration-700 group-hover:scale-110 ${PILLAR_ORB[pillar.id]}`}
        aria-hidden
      />

      <div className="relative z-[1] flex min-h-0 flex-1 flex-col p-3.5 sm:p-4">
        <header className="mb-2.5 flex shrink-0 items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-lg leading-none" aria-hidden>
                {PILLAR_EMOJI[pillar.id]}
              </span>
              <h2 className="truncate text-lg font-semibold tracking-tight text-[var(--dash-ink)] sm:text-xl">
                {pillar.title}
              </h2>
            </div>
          </div>
          <Link
            href={pillar.href}
            className="shrink-0 cursor-pointer rounded-full border border-white/70 bg-white/80 px-3 py-1.5 text-[11px] font-semibold text-[var(--dash-primary)] shadow-sm transition hover:bg-white"
          >
            Ouvrir →
          </Link>
        </header>

        <div className="grid grid-cols-2 gap-1.5">
          <AnimatePresence mode="popLayout">
            {visible.length === 0 ? (
              <p className="col-span-2 text-xs text-[var(--dash-mid)]">Aucun module accessible.</p>
            ) : (
              visible.map((s) => (
                <ShortcutTile
                  key={s.id}
                  item={{
                    ...s,
                    rich: Boolean(s.rich && s.tone !== "neutral"),
                  }}
                  fullWidth={Boolean(s.rich && s.tone !== "neutral")}
                  notifCount={notificationCountForShortcut(s, notifications)}
                  highlight={Boolean(
                    pulseKey && s.rich && s.tone !== "neutral" && pulseKey.includes(s.id),
                  )}
                />
              ))
            )}
          </AnimatePresence>
        </div>
      </div>
    </motion.article>
  );
}

export default function DashboardPillars({
  categories,
  shortcuts,
  notifications = [],
  pulseKey,
  roles = [],
  orgAdmin = false,
}: Props) {
  const pillars = DASHBOARD_PILLARS.filter((p) =>
    pillarHasVisibleModules(p, categories, roles, { orgAdmin }),
  );

  const pruned = useCallback(
    (id: DashboardPillarId) => {
      const pillar = DASHBOARD_PILLARS.find((p) => p.id === id);
      const moduleSet = new Set(
        pillar ? moduleIdsForPillarViewer(pillar, roles, { orgAdmin }) : [],
      );
      const list = shortcuts.filter(
        (s) =>
          (s.pillarId === id || moduleSet.has(s.moduleId)) &&
          s.id !== "rh-home" &&
          !s.pillarOnly,
      );
      const richModules = new Set(list.filter((s) => s.rich).map((s) => s.moduleId));
      return list.filter((s) => {
        if (s.rich) return true;
        if (
          richModules.has(s.moduleId) &&
          [
            "travels",
            "internat",
            "stages",
            "absences",
            "demandes-hse",
            "photocopies-couleur",
            "prof-room",
          ].includes(s.moduleId)
        ) {
          return false;
        }
        return true;
      });
    },
    [shortcuts, roles, orgAdmin],
  );

  if (pillars.length === 0) return null;

  return (
    <div className="grid grid-cols-1 gap-3 pb-6 sm:grid-cols-2 lg:gap-4">
      {pillars.map((pillar, i) => (
        <PillarCard
          key={pillar.id}
          pillar={pillar}
          shortcuts={pruned(pillar.id)}
          notifications={notifications}
          index={i}
          pulseKey={pulseKey}
        />
      ))}
    </div>
  );
}
