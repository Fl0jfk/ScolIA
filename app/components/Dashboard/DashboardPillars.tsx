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
import { PILLAR_EDGE, PILLAR_ORB, PILLAR_WASH } from "@/app/lib/dashboard-pillar-visual";

type Props = {
  categories: DashboardCategory[];
  shortcuts: DashboardShortcut[];
  notifications?: DashboardNotification[];
  pulseKey?: string;
  roles?: string[];
  orgAdmin?: boolean;
  accessibleModuleIds?: Set<string>;
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
      className="col-span-1"
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
        <div
          className="relative min-h-[3.25rem] overflow-hidden rounded-xl px-2 py-2"
          style={{ backgroundColor: bg, color: fg }}
        >
          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={slide.id}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.28 }}
              className="flex items-center gap-1.5"
            >
              <span
                className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-xs leading-none"
                style={{ backgroundColor: "rgba(255,255,255,0.2)" }}
                aria-hidden
              >
                {emoji}
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-1">
                  <p className="truncate text-[11px] font-semibold tracking-tight">{title}</p>
                  <NotificationCountBadge count={notifCount} />
                </div>
                <p className="mt-0.5 truncate text-[9px] font-semibold leading-snug opacity-90">
                  {[slide.label, slide.detail].filter(Boolean).join(" · ")}
                </p>
              </div>
            </motion.div>
          </AnimatePresence>
          <span className="sr-only">{chip}</span>
        </div>
      </Link>
    </motion.div>
  );
}

function badgeCountFallback(item: DashboardShortcut, notifCount: number): number {
  if (notifCount > 0) return notifCount;
  if (item.tone !== "warn" && item.tone !== "action") return 0;
  const fromBadge = item.badge?.match(/^(\d+)/);
  return fromBadge ? Number(fromBadge[1]) : 0;
}

/** Vraie notif à traiter — pas un simple libellé marketing (Paramètres, QR, « demander une… »). */
function isActionableSignal(item: DashboardShortcut, notifCount: number): boolean {
  if (notifCount > 0) return true;
  if (item.tone === "warn") return true;
  if (item.tone === "action" && /^\d+/.test(item.badge?.trim() || "")) return true;
  return false;
}

type TileFace = {
  id: string;
  line: string;
  kind: "label" | "alert";
};

function buildSignalFaces(item: DashboardShortcut): TileFace[] {
  const faces: TileFace[] = [{ id: "label", line: item.label, kind: "label" }];
  const detail = item.detail?.trim();
  const badge = item.badge?.trim();
  // Face alerte : le détail (qui / quoi) porte l’info ; le badge complète s’il ajoute du sens.
  if (detail) {
    faces.push({ id: "detail", line: detail, kind: "alert" });
    if (badge && !detail.includes(badge) && !/^\d+\s/.test(detail)) {
      faces.push({ id: "badge", line: badge, kind: "alert" });
    }
  } else if (badge) {
    faces.push({ id: "badge", line: badge, kind: "alert" });
  }
  return faces;
}

function SignalShortcutTile({
  item,
  highlight,
  notifCount = 0,
}: {
  item: DashboardShortcut;
  highlight?: boolean;
  notifCount?: number;
}) {
  const displayCount = badgeCountFallback(item, notifCount);
  const actionable = isActionableSignal(item, notifCount);
  const emoji = item.emoji || MODULE_EMOJI[item.moduleId] || "›";
  const staticDetail = !actionable ? item.detail?.trim() || null : null;
  const faces = actionable
    ? buildSignalFaces(item)
    : [{ id: "label", line: item.label, kind: "label" as const }];
  const [faceIndex, setFaceIndex] = useState(0);
  const faceKey = faces.map((f) => `${f.id}:${f.line}`).join("|");

  useEffect(() => {
    setFaceIndex(0);
  }, [faceKey]);

  useEffect(() => {
    if (!actionable || faces.length <= 1) return;
    const id = window.setInterval(() => {
      setFaceIndex((i) => (i + 1) % faces.length);
    }, 3200);
    return () => window.clearInterval(id);
  }, [actionable, faces.length, faceKey]);

  const face = faces[faceIndex] ?? faces[0]!;
  const warnTone = item.tone === "warn";
  const alertFace = actionable && face.kind === "alert";
  const solidAlert = warnTone ? "#f43f5e" : "#f59e0b"; // rose-500 / amber-500
  const alertMuted = "text-white/90";

  return (
    <motion.div
      layout
      className="col-span-1"
      initial={highlight ? { scale: 0.97, opacity: 0.45 } : false}
      animate={{ scale: 1, opacity: 1 }}
      transition={{ type: "spring", stiffness: 380, damping: 28 }}
    >
      <Link
        href={item.href}
        className={`group relative flex min-h-[3.25rem] cursor-pointer items-center gap-1.5 overflow-hidden rounded-xl border px-2 py-2 transition hover:-translate-y-0.5 ${
          actionable
            ? "border-slate-200/80 shadow-[0_10px_28px_-20px_rgba(15,23,42,0.45)]"
            : "border-transparent bg-white/40 hover:border-white/70 hover:bg-white/70"
        }`}
      >
        {actionable ? (
          <motion.span
            aria-hidden
            className="pointer-events-none absolute inset-0 rounded-xl"
            initial={false}
            animate={{
              backgroundColor: alertFace ? solidAlert : "#ffffff",
            }}
            transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
          />
        ) : null}
        {highlight && actionable ? (
          <motion.span
            className="pointer-events-none absolute inset-0 z-[1] bg-white/35"
            initial={{ opacity: 0.85 }}
            animate={{ opacity: 0 }}
            transition={{ duration: 1.2 }}
          />
        ) : null}
        <span
          className={`relative z-[1] flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-lg ring-1 ${
            actionable
              ? alertFace
                ? "bg-white/20 ring-white/40"
                : warnTone
                  ? "bg-rose-50 ring-rose-200/90"
                  : "bg-amber-50 ring-amber-200/90"
              : "bg-[color:var(--dash-soft-muted)]/90 ring-[color:var(--dash-border)]/60"
          }`}
        >
          <span className="text-xs leading-none" aria-hidden>
            {emoji}
          </span>
        </span>
        <div className="relative z-[1] min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-1">
            <div className="relative min-h-[1.15rem] min-w-0 flex-1 overflow-hidden">
              {actionable ? (
                <AnimatePresence mode="wait" initial={false}>
                  <motion.p
                    key={face.id}
                    initial={{ opacity: 0, y: 10, filter: "blur(2px)" }}
                    animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
                    exit={{ opacity: 0, y: -10, filter: "blur(2px)" }}
                    transition={{ duration: 0.32, ease: [0.22, 1, 0.36, 1] }}
                    className={`truncate text-[11px] font-semibold leading-tight tracking-tight ${
                      alertFace ? "text-white" : "text-[var(--dash-ink)]"
                    }`}
                    title={face.line}
                  >
                    {face.line}
                  </motion.p>
                </AnimatePresence>
              ) : (
                <p
                  className="truncate text-[11px] font-semibold leading-tight tracking-tight text-[var(--dash-ink)]"
                  title={item.label}
                >
                  {item.label}
                </p>
              )}
            </div>
            <NotificationCountBadge count={displayCount} />
          </div>
          {actionable && item.detail?.trim() && face.kind === "label" ? (
            <p className="mt-0.5 truncate text-[9px] font-medium leading-snug text-[var(--dash-mid)]" title={item.detail}>
              {item.detail}
            </p>
          ) : actionable && face.kind === "alert" && item.label ? (
            <p className={`mt-0.5 truncate text-[9px] font-medium leading-snug ${alertMuted}`} title={item.label}>
              {item.label}
            </p>
          ) : staticDetail ? (
            <p className="mt-0.5 truncate text-[9px] leading-snug text-[var(--dash-mid)]" title={staticDetail}>
              {staticDetail}
            </p>
          ) : null}
        </div>
      </Link>
    </motion.div>
  );
}

function ShortcutTile({
  item,
  highlight,
  notifCount = 0,
}: {
  item: DashboardShortcut;
  highlight?: boolean;
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
        notifCount={badgeCountFallback(item, notifCount)}
      />
    );
  }

  return <SignalShortcutTile item={item} highlight={highlight} notifCount={notifCount} />;
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
  // Chaque tuile = demi-ligne : on peut en montrer davantage (plus de modules visibles).
  const plainBudget = Math.max(pinned.length, Math.max(0, 12 - rich.length));
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
        className={`pointer-events-none absolute inset-y-0 left-0 z-0 w-1.5 ${PILLAR_EDGE[pillar.id]}`}
        aria-hidden
      />
      <div
        className={`pointer-events-none absolute inset-y-0 left-0 z-0 w-[85%] bg-gradient-to-r ${PILLAR_WASH[pillar.id]} to-transparent`}
        aria-hidden
      />
      <div
        className={`pointer-events-none absolute -left-6 top-1/2 z-0 h-48 w-48 -translate-y-1/2 rounded-full blur-3xl transition duration-700 group-hover:scale-110 ${PILLAR_ORB[pillar.id]}`}
        aria-hidden
      />
      <div
        className={`pointer-events-none absolute -right-10 -top-12 z-0 h-40 w-40 rounded-full blur-3xl opacity-70 transition duration-700 group-hover:scale-110 ${PILLAR_ORB[pillar.id]}`}
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
  accessibleModuleIds,
}: Props) {
  const pillars = DASHBOARD_PILLARS.filter((p) =>
    pillarHasVisibleModules(p, categories, roles, { orgAdmin, accessibleModuleIds }),
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
