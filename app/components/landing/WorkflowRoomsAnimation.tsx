"use client";

import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  WORKFLOW_ANIMATION_BODY,
  WORKFLOW_ANIMATION_INNER,
  WORKFLOW_ANIMATION_SHELL,
} from "@/app/lib/marketing-theme";

const PHASES = ["grille", "creneau", "reserver", "confirme"] as const;
type Phase = (typeof PHASES)[number];

const PHASE_MS = 2200;

const HOURS = ["8h", "9h", "10h", "11h"];
const DAYS = ["Lun", "Mar", "Mer", "Jeu", "Ven"];

const BOOK_DAY = 1;
const BOOK_HOUR = 2;

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

  const showBooking = phase === "creneau" || phase === "reserver" || phase === "confirme";
  const showFilled = phase === "confirme";

  return (
    <div
      className={`${WORKFLOW_ANIMATION_SHELL} border border-violet-400/25 bg-gradient-to-br from-[#3B2F5F] via-[#4C3D7A] to-[#2D2448] shadow-violet-900/25`}
    >
      <div className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-violet-400/20 blur-2xl" />
      <div className={`${WORKFLOW_ANIMATION_INNER} bg-[#1A1228]/50`}>
        <div className="mb-3 flex shrink-0 items-center gap-2 border-b border-white/10 pb-3">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-violet-400/80" />
                      <span className="ml-2 text-xs font-semibold text-violet-100/80">
            Réservation de salles
          </span>
          <motion.span
            key={tick}
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            className="ml-auto rounded-full bg-violet-400/20 px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-violet-200"
          >
            {phase === "confirme" ? "Confirmé" : "Temps réel"}
          </motion.span>
        </div>

        <div className={`${WORKFLOW_ANIMATION_BODY} grid gap-3 sm:grid-cols-[1.2fr_1fr]`}>
          <div className="flex flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2.5">
            <p className="mb-2 shrink-0 text-[10px] font-bold uppercase tracking-wider text-violet-200/60">
              Planning hebdo
            </p>
            <div className="min-h-0 flex-1 overflow-hidden">
              <table className="w-full border-collapse text-[9px]">
                <thead>
                  <tr>
                    <th className="p-1 text-left text-white/30" />
                    {DAYS.map((d) => (
                      <th key={d} className="p-1 font-bold text-violet-200/70">
                        {d}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {HOURS.map((h, hi) => (
                    <tr key={h}>
                      <td className="p-1 font-semibold text-white/40">{h}</td>
                      {DAYS.map((_, di) => {
                        const isTarget = di === BOOK_DAY && hi === BOOK_HOUR;
                        const isLit = showBooking && isTarget;
                        const isFilled = showFilled && isTarget;
                        return (
                          <td key={di} className="p-0.5">
                            <div
                              className={`flex h-7 items-center justify-center rounded-md border-2 transition-all duration-300 ${
                                isFilled
                                  ? "border-violet-300/80 bg-violet-400/85"
                                  : isLit
                                    ? "border-violet-300/80 bg-violet-300/45"
                                    : "border-violet-300/0 bg-white/6"
                              } ${phase === "creneau" && isTarget ? "ring-2 ring-violet-300/40" : ""}`}
                            >
                              {isFilled ? (
                                <span className="truncate px-0.5 text-[8px] font-black text-white">
                                  Maths 6A
                                </span>
                              ) : null}
                            </div>
                          </td>
                        );
                      })}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          <div className="flex flex-col rounded-2xl border border-white/10 bg-white/5 p-3">
            <p className="shrink-0 text-[10px] font-bold uppercase tracking-wider text-violet-200/60">
              {phase === "grille" ? "Créneaux libres" : "Nouvelle réservation"}
            </p>
            <div className="relative mt-2 min-h-0 flex-1">
              <AnimatePresence mode="wait">
                {phase === "grille" ? (
                  <motion.p
                    key="idle"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="text-[11px] leading-relaxed text-violet-100/45"
                  >
                    Cliquez sur un créneau libre pour réserver…
                  </motion.p>
                ) : (
                  <motion.div
                    key="form"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-2"
                  >
                    <div className="rounded-lg bg-white/10 px-2.5 py-2">
                      <p className="text-[10px] text-violet-200/50">Matière</p>
                      <div className="mt-0.5 flex items-center gap-2">
                        <span className="h-2.5 w-2.5 rounded-full bg-violet-400" />
                        <span className="text-xs font-bold text-white">Mathématiques</span>
                      </div>
                    </div>
                    <div className="rounded-lg bg-white/10 px-2.5 py-2">
                      <p className="text-[10px] text-violet-200/50">Classe</p>
                      <p className="text-xs font-bold text-white">6A · Collège</p>
                    </div>
                    <div
                      className={`rounded-lg bg-fuchsia-500/15 px-2.5 py-2 ring-1 ring-fuchsia-400/30 transition-opacity duration-300 ${
                        phase === "reserver" || phase === "confirme" ? "opacity-100" : "opacity-0"
                      }`}
                    >
                      <p className="text-[10px] text-fuchsia-200/70">Récurrence</p>
                      <p className="text-xs font-bold text-white">Toutes les semaines · jusqu&apos;en juin</p>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
            <p
              className={`mt-2 shrink-0 text-center text-xs font-black transition-opacity duration-300 ${
                phase === "confirme" ? "text-[#4ADE80] opacity-100" : "text-transparent opacity-0"
              }`}
            >
              ✓ Confirmé — e-mail de rappel envoyé
            </p>
          </div>
        </div>

        <div className="mt-3 flex shrink-0 justify-center gap-1.5">
          {PHASES.map((p) => (
            <span
              key={p}
              className={`h-1.5 rounded-full transition-all duration-500 ${
                phase === p ? "w-6 bg-violet-400" : "w-1.5 bg-white/20"
              }`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
