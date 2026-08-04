"use client";

import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  WORKFLOW_ANIMATION_BODY,
  WORKFLOW_ANIMATION_INNER,
  WORKFLOW_ANIMATION_SHELL,
} from "@/app/lib/marketing-theme";

const PHASES = ["grille", "occupe", "libre", "select", "conflit", "confirme", "live"] as const;
type Phase = (typeof PHASES)[number];
const PHASE_MS = 1900;

const HOURS = ["8h", "9h", "10h", "11h"];
const DAYS = ["Lun", "Mar", "Mer", "Jeu", "Ven"];

/** Créneaux déjà occupés (jour, heure) */
const BUSY: Array<[number, number]> = [
  [0, 0],
  [0, 1],
  [2, 1],
  [3, 0],
  [4, 2],
];

const BOOK_DAY = 1;
const BOOK_HOUR = 2;
/** Créneau en conflit potentiel (même jour, autre heure affichée en flash) */
const CONFLICT_DAY = 1;
const CONFLICT_HOUR = 1;

const PHASE_LABEL: Record<Phase, string> = {
  grille: "Planning",
  occupe: "Occupés",
  libre: "Dispos",
  select: "Sélection",
  conflit: "Conflit",
  confirme: "Confirmé",
  live: "Live",
};

const TOAST: Record<Phase, string> = {
  grille: "Planning hebdomadaire S12",
  occupe: "Créneaux déjà réservés",
  libre: "Créneaux libres mis en avant",
  select: "Mar · 10h — Salle 204",
  conflit: "Conflit évité · créneau OK",
  confirme: "Maths 6A confirmé",
  live: "Temps réel · sync calendrier",
};

function isBusy(di: number, hi: number) {
  return BUSY.some(([d, h]) => d === di && h === hi);
}

export default function WorkflowRoomsAnimation() {
  const [phase, setPhase] = useState<Phase>("grille");
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => {
      setPhase((p) => PHASES[(PHASES.indexOf(p) + 1) % PHASES.length]);
      setTick((t) => t + 1);
    }, PHASE_MS);
    return () => clearInterval(id);
  }, []);

  const highlightBusy = phase === "occupe" || phase === "libre";
  const highlightFree = phase === "libre" || phase === "select" || phase === "conflit";
  const showSelect =
    phase === "select" || phase === "conflit" || phase === "confirme" || phase === "live";
  const showFilled = phase === "confirme" || phase === "live";
  const flashConflict = phase === "conflit";

  return (
    <div
      className={`${WORKFLOW_ANIMATION_SHELL} border border-violet-400/25 bg-gradient-to-br from-[#3B2F5F] via-[#4C3D7A] to-[#2D2448] shadow-violet-900/25`}
    >
      <div className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-violet-400/20 blur-2xl" />
      <div className={`${WORKFLOW_ANIMATION_INNER} bg-[#1A1228]/50`}>
        <div className="mb-2 flex shrink-0 items-center gap-2 border-b border-white/10 pb-2.5">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-violet-400/80" />
          <span className="ml-2 text-xs font-semibold text-violet-100/80">Réservation de salles</span>
          <motion.span
            key={tick}
            initial={{ opacity: 0, scale: 0.85 }}
            animate={{ opacity: 1, scale: 1 }}
            className="ml-auto rounded-full bg-violet-400/20 px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-violet-200"
          >
            {PHASE_LABEL[phase]}
          </motion.span>
        </div>

        <div className={`${WORKFLOW_ANIMATION_BODY} grid gap-2.5 sm:grid-cols-[1.25fr_1fr]`}>
          <div className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2">
            <div className="mb-1.5 flex shrink-0 items-center justify-between">
              <p className="text-[10px] font-bold uppercase tracking-wider text-violet-200/60">
                Planning hebdo
              </p>
              {(phase === "live" || phase === "confirme") && (
                <motion.span
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className="rounded-full bg-emerald-400/20 px-1.5 py-0.5 text-[8px] font-black uppercase tracking-wide text-emerald-200"
                >
                  Temps réel
                </motion.span>
              )}
            </div>
            <div className="min-h-0 flex-1 overflow-hidden">
              <table className="w-full border-collapse text-[9px]">
                <thead>
                  <tr>
                    <th className="p-0.5 text-left text-white/30" />
                    {DAYS.map((d) => (
                      <th key={d} className="p-0.5 font-bold text-violet-200/70">
                        {d}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {HOURS.map((h, hi) => (
                    <tr key={h}>
                      <td className="p-0.5 font-semibold text-white/40">{h}</td>
                      {DAYS.map((_, di) => {
                        const busy = isBusy(di, hi);
                        const isTarget = di === BOOK_DAY && hi === BOOK_HOUR;
                        const isConflictCell = di === CONFLICT_DAY && hi === CONFLICT_HOUR;
                        const isLit = showSelect && isTarget;
                        const isFilled = showFilled && isTarget;

                        let cellClass =
                          "border-violet-300/0 bg-white/6";
                        if (isFilled) {
                          cellClass = "border-violet-300/80 bg-violet-400/85";
                        } else if (flashConflict && isConflictCell) {
                          cellClass = "border-rose-400/80 bg-rose-500/50";
                        } else if (isLit) {
                          cellClass = "border-violet-300/80 bg-violet-300/45";
                        } else if (busy && highlightBusy) {
                          cellClass = "border-amber-400/40 bg-amber-400/35";
                        } else if (!busy && highlightFree && !isTarget) {
                          cellClass = "border-emerald-400/35 bg-emerald-400/20";
                        } else if (busy) {
                          cellClass = "border-white/5 bg-white/10";
                        }

                        return (
                          <td key={di} className="p-0.5">
                            <motion.div
                              layout
                              animate={
                                phase === "select" && isTarget
                                  ? { scale: [1, 1.08, 1] }
                                  : flashConflict && isConflictCell
                                    ? { opacity: [1, 0.5, 1] }
                                    : { scale: 1, opacity: 1 }
                              }
                              transition={{ duration: 0.7 }}
                              className={`flex h-6 items-center justify-center rounded-md border-2 transition-colors duration-300 sm:h-7 ${cellClass} ${
                                phase === "select" && isTarget ? "ring-2 ring-violet-300/50" : ""
                              }`}
                            >
                              {isFilled ? (
                                <span className="truncate px-0.5 text-[7px] font-black text-white sm:text-[8px]">
                                  Maths 6A
                                </span>
                              ) : flashConflict && isConflictCell ? (
                                <span className="text-[7px] font-black text-rose-100">!</span>
                              ) : busy && highlightBusy ? (
                                <span className="h-1 w-1 rounded-full bg-amber-200/80" />
                              ) : null}
                            </motion.div>
                          </td>
                        );
                      })}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          <div className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2.5">
            <p className="shrink-0 text-[10px] font-bold uppercase tracking-wider text-violet-200/60">
              {phase === "grille" || phase === "occupe" || phase === "libre"
                ? "Disponibilités"
                : "Nouvelle réservation"}
            </p>
            <div className="relative mt-1.5 min-h-0 flex-1">
              <AnimatePresence mode="wait">
                {phase === "grille" || phase === "occupe" ? (
                  <motion.p
                    key="idle"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="text-[11px] leading-relaxed text-violet-100/45"
                  >
                    {phase === "occupe"
                      ? "Les créneaux orange sont déjà pris…"
                      : "Cliquez sur un créneau libre pour réserver…"}
                  </motion.p>
                ) : phase === "libre" ? (
                  <motion.div
                    key="free"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    <p className="text-[11px] font-bold text-emerald-200">3 créneaux libres · Mar</p>
                    <p className="text-[10px] text-violet-100/50">9h · 10h · 11h — Salle 204</p>
                  </motion.div>
                ) : flashConflict ? (
                  <motion.div
                    key="conflict"
                    initial={{ opacity: 0, scale: 0.96 }}
                    animate={{ opacity: 1, scale: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-2"
                  >
                    <div className="rounded-lg bg-rose-500/25 px-2.5 py-2 ring-1 ring-rose-400/40">
                      <p className="text-[10px] font-black text-rose-100">Conflit détecté</p>
                      <p className="mt-0.5 text-[10px] text-rose-100/70">
                        Mar 9h déjà pris — 10h disponible
                      </p>
                    </div>
                    <p className="text-center text-[10px] font-bold text-emerald-300">
                      Créneau 10h conservé ✓
                    </p>
                  </motion.div>
                ) : (
                  <motion.div
                    key="form"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    <div className="rounded-lg bg-white/10 px-2.5 py-1.5">
                      <p className="text-[10px] text-violet-200/50">Matière</p>
                      <div className="mt-0.5 flex items-center gap-2">
                        <span className="h-2 w-2 rounded-full bg-violet-400" />
                        <span className="text-xs font-bold text-white">Mathématiques</span>
                      </div>
                    </div>
                    <div className="rounded-lg bg-white/10 px-2.5 py-1.5">
                      <p className="text-[10px] text-violet-200/50">Classe · Salle</p>
                      <p className="text-xs font-bold text-white">6A · Salle 204</p>
                    </div>
                    {(phase === "confirme" || phase === "live") && (
                      <motion.div
                        initial={{ opacity: 0, y: 4 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="rounded-lg bg-fuchsia-500/15 px-2.5 py-1.5 ring-1 ring-fuchsia-400/30"
                      >
                        <p className="text-[10px] text-fuchsia-200/70">Récurrence</p>
                        <p className="text-[11px] font-bold text-white">
                          Chaque semaine · jusqu&apos;en juin
                        </p>
                      </motion.div>
                    )}
                    {showFilled ? (
                      <p className="pt-1 text-center text-[11px] font-black text-[#4ADE80]">
                        ✓ Confirmé — rappel envoyé
                      </p>
                    ) : null}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>
        </div>

        <div className="mt-2 flex shrink-0 flex-col items-center gap-2">
          <AnimatePresence mode="wait">
            <motion.p
              key={phase}
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -4 }}
              className={`rounded-full px-3 py-1 text-[11px] font-bold ${
                phase === "live" || phase === "confirme"
                  ? "bg-emerald-400/20 text-[#4ADE80] ring-1 ring-emerald-300/40"
                  : "bg-white/10 text-violet-100/85"
              }`}
            >
              {TOAST[phase]}
            </motion.p>
          </AnimatePresence>
          <div className="flex gap-1.5">
            {PHASES.map((p) => (
              <span
                key={p}
                className={`h-1.5 rounded-full transition-all duration-500 ${
                  phase === p ? "w-5 bg-violet-400" : "w-1.5 bg-white/20"
                }`}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
