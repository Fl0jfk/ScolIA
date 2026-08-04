"use client";

import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  WORKFLOW_ANIMATION_BODY,
  WORKFLOW_ANIMATION_INNER,
  WORKFLOW_ANIMATION_SHELL,
} from "@/app/lib/marketing-theme";

const PHASES = ["absence", "calendrier", "onboarding", "ok"] as const;
type Phase = (typeof PHASES)[number];
const PHASE_MS = 2400;

export default function WorkflowAbsencesAnimation() {
  const [phase, setPhase] = useState<Phase>("absence");
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => {
      setPhase((p) => PHASES[(PHASES.indexOf(p) + 1) % PHASES.length]);
      setTick((t) => t + 1);
    }, PHASE_MS);
    return () => clearInterval(id);
  }, []);

  return (
    <div
      className={`${WORKFLOW_ANIMATION_SHELL} border border-rose-400/20 bg-gradient-to-br from-[#5C2D3A] via-[#6B3A4A] to-[#452530] shadow-rose-900/20`}
    >
      <div className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-rose-400/15 blur-2xl" />
      <div className={`${WORKFLOW_ANIMATION_INNER} bg-[#2A1520]/50`}>
        <div className="mb-3 flex shrink-0 items-center gap-2 border-b border-white/10 pb-3">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-rose-400/80" />
          <span className="ml-2 text-xs font-semibold text-rose-100/80">Ressources humaines</span>
          <motion.span
            key={tick}
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            className="ml-auto rounded-full bg-rose-400/20 px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-rose-200"
          >
            {phase === "onboarding" || phase === "ok" ? "Nouveau salarié" : "Absences"}
          </motion.span>
        </div>

        <div className={`${WORKFLOW_ANIMATION_BODY} grid gap-3 sm:grid-cols-2`}>
          <div className="flex flex-col rounded-2xl border border-white/10 bg-white/5 p-3">
            <p className="text-[10px] font-bold uppercase tracking-wider text-rose-200/60">
              {phase === "onboarding" || phase === "ok" ? "Arrivée" : "Se déclarer"}
            </p>
            <div className="relative mt-2 min-h-[7rem] flex-1">
              <AnimatePresence mode="wait">
                {phase === "absence" || phase === "calendrier" ? (
                  <motion.div
                    key="abs"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-2"
                  >
                    <div className="rounded-lg bg-white/10 px-2.5 py-2">
                      <p className="text-[10px] text-rose-200/50">Période</p>
                      <p className="text-xs font-bold text-white">Mardi 14 · 8h30 → 12h30</p>
                    </div>
                    <div className="rounded-lg bg-white/10 px-2.5 py-2">
                      <p className="text-[10px] text-rose-200/50">Motif</p>
                      <p className="text-xs text-white/90">Rendez-vous médical</p>
                    </div>
                    {phase === "absence" ? (
                      <div className="rounded-lg bg-rose-500/30 py-2 text-center text-[11px] font-black text-white">
                        Envoyer
                      </div>
                    ) : (
                      <p className="text-center text-xs font-bold text-[#4ADE80]">✓ Sur le calendrier</p>
                    )}
                  </motion.div>
                ) : (
                  <motion.div
                    key="onb"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-2"
                  >
                    <div className="flex items-center gap-2 rounded-lg bg-white/10 px-2.5 py-2">
                      <span className="flex h-8 w-8 items-center justify-center rounded-full bg-rose-400/30 text-xs font-black text-white">
                        CM
                      </span>
                      <div>
                        <p className="text-xs font-bold text-white">Camille Moreau</p>
                        <p className="text-[10px] text-rose-200/55">Professeure · Collège</p>
                      </div>
                    </div>
                    <ul className="space-y-1 text-[10px] text-rose-100/80">
                      <li className="flex justify-between rounded bg-white/5 px-2 py-1">
                        <span>Invitation envoyée</span>
                        <span className="text-[#4ADE80]">✓</span>
                      </li>
                      <li className="flex justify-between rounded bg-white/5 px-2 py-1">
                        <span>Documents à fournir</span>
                        <span className={phase === "ok" ? "text-[#4ADE80]" : "text-amber-200"}>
                          {phase === "ok" ? "✓" : "…"}
                        </span>
                      </li>
                      <li className="flex justify-between rounded bg-white/5 px-2 py-1">
                        <span>Compte créé</span>
                        <span className={phase === "ok" ? "text-[#4ADE80]" : "text-white/30"}>
                          {phase === "ok" ? "✓" : "—"}
                        </span>
                      </li>
                    </ul>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>

          <div className="flex flex-col rounded-2xl border border-white/10 bg-white/5 p-3">
            <p className="text-[10px] font-bold uppercase tracking-wider text-rose-200/60">
              Pilotage RH
            </p>
            <ul className="mt-2 flex-1 space-y-1.5">
              {[
                { label: "Absences du jour", value: phase === "calendrier" ? "3" : "2", show: true },
                { label: "À valider", value: phase === "absence" ? "1" : "0", show: true },
                {
                  label: "Onboardings ouverts",
                  value: phase === "onboarding" || phase === "ok" ? "1" : "0",
                  show: true,
                },
                {
                  label: "Dossiers à jour",
                  value: phase === "ok" ? "Oui" : "—",
                  show: phase === "ok" || phase === "onboarding",
                },
              ].map((row) => (
                <li
                  key={row.label}
                  className="flex items-center justify-between rounded-lg bg-white/5 px-2.5 py-1.5"
                >
                  <span className="text-[11px] text-rose-100/80">{row.label}</span>
                  <span className="text-xs font-black text-white">{row.value}</span>
                </li>
              ))}
            </ul>
          </div>
        </div>

        <div className="mt-3 flex justify-center gap-1.5">
          {PHASES.map((p) => (
            <span
              key={p}
              className={`h-1.5 rounded-full transition-all ${
                phase === p ? "w-6 bg-rose-300" : "w-1.5 bg-white/20"
              }`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
