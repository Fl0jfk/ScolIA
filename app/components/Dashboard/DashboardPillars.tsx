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

const PILLAR_EMOJI: Record<DashboardPillarId, string> = {
  eleves: "🎒",
  rh: "👥",
  etablissement: "🏫",
  services: "🛠️",
};

const MODULE_FALLBACK_EMOJI: Record<string, string> = {
  travels: "🚌",
  internat: "🌙",
  stages: "📝",
  "agent-ia-ocr": "📄",
  certificates: "🏅",
  absences: "📅",
  "demandes-hse": "⏱️",
  rh: "👥",
  "prof-room": "🚪",
  "requests-staff": "📨",
  "photocopies-couleur": "🖨️",
  documents: "☁️",
  toolbox: "🧰",
  covoiturage: "🚗",
  channels: "💬",
  assistance: "🆘",
  organigramme: "🗺️",
  "conformite-rgpd": "🔒",
  "chatbot-knowledge": "🧠",
  "domain-planning": "📚",
};

function ShortcutTile({
  item,
  highlight,
  iconSrc,
  fullWidth,
}: {
  item: DashboardShortcut;
  highlight?: boolean;
  iconSrc?: string;
  fullWidth?: boolean;
}) {
  const emoji = MODULE_FALLBACK_EMOJI[item.moduleId] || "›";

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
        className={`group relative flex !cursor-pointer items-center gap-2 overflow-hidden rounded-xl border transition hover:-translate-y-0.5 ${
          item.rich
            ? "border-white/70 bg-white/80 px-2.5 py-2.5 shadow-[0_10px_30px_-22px_rgba(15,23,42,0.45)] backdrop-blur-md hover:bg-white"
            : "border-transparent bg-white/40 px-2 py-2 hover:border-white/70 hover:bg-white/70"
        }`}
        style={{ cursor: "pointer" }}
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
          {iconSrc ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img src={iconSrc} alt="" className="h-full w-full object-cover" />
          ) : (
            <span className="text-sm leading-none">{emoji}</span>
          )}
        </span>
        <div className="relative min-w-0 flex-1">
          <p className="truncate text-[12px] font-semibold tracking-tight text-[var(--dash-ink)]">
            {item.label}
          </p>
          {item.rich && item.detail ? (
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
  index,
  pulseKey,
  iconByModule,
}: {
  pillar: DashboardPillarDef;
  shortcuts: DashboardShortcut[];
  index: number;
  pulseKey?: string;
  iconByModule: Map<string, string>;
}) {
  // Signaux d'abord (pleine largeur), puis raccourcis classiques
  const rich = shortcuts.filter((s) => s.rich);
  const plain = shortcuts.filter((s) => !s.rich);
  // Budget viewport : garder assez pour 2 colonnes sans scroll interne
  const plainBudget = Math.max(0, 6 - rich.length * 2);
  const visiblePlain = plain.slice(0, plainBudget);
  const visible = [...rich, ...visiblePlain];

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

      <div className="relative flex min-h-0 flex-1 flex-col p-3.5 sm:p-4">
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
            <p className="mt-0.5 text-[11px] leading-snug text-[var(--dash-mid)]">
              {pillar.description}
            </p>
          </div>
          <Link
            href={pillar.href}
            className="shrink-0 !cursor-pointer rounded-full border border-white/70 bg-white/70 px-3 py-1.5 text-[11px] font-semibold text-[var(--dash-primary)] shadow-sm backdrop-blur transition hover:bg-white"
            style={{ cursor: "pointer" }}
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
                  item={s}
                  fullWidth={Boolean(s.rich)}
                  iconSrc={iconByModule.get(s.moduleId)}
                  highlight={Boolean(pulseKey && s.rich && pulseKey.includes(s.id))}
                />
              ))
            )}
          </AnimatePresence>
        </div>
      </div>
    </motion.article>
  );
}

export default function DashboardPillars({ categories, shortcuts, pulseKey }: Props) {
  const pillars = DASHBOARD_PILLARS.filter((p) => pillarHasVisibleModules(p, categories));
  const iconByModule = new Map(categories.map((c) => [c.moduleId, c.img]));
  const rhImg = iconByModule.get("rh");
  if (rhImg) {
    iconByModule.set("absences", rhImg);
    iconByModule.set("demandes-hse", rhImg);
  }

  const pruned = (id: DashboardPillarId) => {
    const list = shortcuts.filter(
      (s) => s.pillarId === id && s.id !== "rh-home" && !s.pillarOnly,
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
          iconByModule={iconByModule}
        />
      ))}
    </div>
  );
}
