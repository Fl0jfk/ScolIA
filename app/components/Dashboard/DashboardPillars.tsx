"use client";

import Link from "next/link";
import { AnimatePresence, motion } from "framer-motion";
import type { DashboardCategory } from "@/app/lib/intranet-modules";
import {
  DASHBOARD_PILLARS,
  pillarHasVisibleModules,
  type DashboardPillarDef,
  type DashboardPillarId,
} from "@/app/lib/dashboard-pillars";
import type { DashboardShortcut } from "@/app/lib/dashboard-signals";

type Props = {
  categories: DashboardCategory[];
  shortcuts: DashboardShortcut[];
  pulseKey?: string;
};

const PILLAR_ORB: Record<DashboardPillarId, string> = {
  eleves: "bg-sky-400/30",
  rh: "bg-violet-400/25",
  etablissement: "bg-amber-400/30",
  services: "bg-emerald-400/30",
};

function ShortcutRow({ item, highlight }: { item: DashboardShortcut; highlight?: boolean }) {
  if (item.rich) {
    return (
      <motion.div
        layout
        initial={highlight ? { scale: 0.96, opacity: 0.4 } : false}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ type: "spring", stiffness: 380, damping: 28 }}
      >
        <Link
          href={item.href}
          className="group relative flex items-start justify-between gap-2 overflow-hidden rounded-2xl border border-white/70 bg-white/70 px-3.5 py-3 shadow-[0_10px_30px_-22px_rgba(15,23,42,0.45)] backdrop-blur-md transition hover:-translate-y-0.5 hover:bg-white/90"
        >
          {highlight ? (
            <motion.span
              className="pointer-events-none absolute inset-0 bg-[color:var(--dash-bright)]/15"
              initial={{ opacity: 0.8 }}
              animate={{ opacity: 0 }}
              transition={{ duration: 1.4 }}
            />
          ) : null}
          <div className="relative min-w-0">
            <p className="truncate text-[13px] font-semibold tracking-tight text-[var(--dash-ink)]">
              {item.label}
            </p>
            {item.detail ? (
              <p className="mt-0.5 line-clamp-2 text-[11px] leading-snug text-[var(--dash-mid)]">
                {item.detail}
              </p>
            ) : null}
          </div>
          {item.badge ? (
            <span className="relative shrink-0 rounded-full bg-[var(--dash-primary)] px-2.5 py-1 text-[10px] font-bold text-white shadow-sm">
              {item.badge}
            </span>
          ) : null}
        </Link>
      </motion.div>
    );
  }

  return (
    <Link
      href={item.href}
      className="flex items-center justify-between gap-2 rounded-xl px-2.5 py-2 text-[13px] font-medium text-[var(--dash-ink)]/90 transition hover:bg-white/55"
    >
      <span className="truncate">{item.label}</span>
      <span className="text-[var(--dash-mid)] opacity-60">›</span>
    </Link>
  );
}

function PillarCard({
  pillar,
  shortcuts,
  index,
  pulseKey,
}: {
  pillar: DashboardPillarDef;
  shortcuts: DashboardShortcut[];
  index: number;
  pulseKey?: string;
}) {
  const rich = shortcuts.filter((s) => s.rich);
  const plain = shortcuts.filter((s) => !s.rich);
  const visible = [...rich, ...plain.slice(0, Math.max(0, 4 - rich.length))];

  return (
    <motion.article
      layout
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, delay: 0.06 * index, ease: [0.22, 1, 0.36, 1] }}
      className="group relative flex min-h-0 flex-col overflow-hidden rounded-[1.5rem] border border-white/55 bg-white/45 shadow-[0_20px_50px_-32px_rgba(15,23,42,0.45)] backdrop-blur-2xl"
    >
      <div
        className={`pointer-events-none absolute -right-8 -top-10 h-36 w-36 rounded-full blur-3xl transition duration-700 group-hover:scale-110 ${PILLAR_ORB[pillar.id]}`}
        aria-hidden
      />

      <div className="relative flex min-h-0 flex-1 flex-col p-4 sm:p-5">
        <header className="mb-3 shrink-0">
          <Link href={pillar.href} className="block">
            <h2 className="text-xl font-semibold tracking-tight text-[var(--dash-ink)] sm:text-[1.35rem]">
              {pillar.title}
            </h2>
            <p className="mt-1 text-[12px] leading-snug text-[var(--dash-mid)]">
              {pillar.description}
            </p>
          </Link>
        </header>

        <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto pr-0.5">
          <AnimatePresence mode="popLayout">
            {visible.length === 0 ? (
              <p className="text-xs text-[var(--dash-mid)]">Aucun module accessible.</p>
            ) : (
              visible.map((s) => (
                <ShortcutRow
                  key={s.id}
                  item={s}
                  highlight={Boolean(pulseKey && s.rich && pulseKey.includes(s.id))}
                />
              ))
            )}
          </AnimatePresence>
        </div>

        <Link
          href={pillar.href}
          className="mt-3 shrink-0 text-center text-[11px] font-semibold text-[var(--dash-primary)] transition hover:opacity-80"
        >
          Ouvrir le module →
        </Link>
      </div>
    </motion.article>
  );
}

export default function DashboardPillars({ categories, shortcuts, pulseKey }: Props) {
  const pillars = DASHBOARD_PILLARS.filter((p) => pillarHasVisibleModules(p, categories));

  const pruned = (id: DashboardPillarId) => {
    const list = shortcuts.filter((s) => s.pillarId === id);
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
  };

  if (pillars.length === 0) return null;

  return (
    <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 sm:grid-cols-2 lg:gap-4">
      {pillars.map((pillar, i) => (
        <PillarCard
          key={pillar.id}
          pillar={pillar}
          shortcuts={pruned(pillar.id)}
          index={i}
          pulseKey={pulseKey}
        />
      ))}
    </div>
  );
}
