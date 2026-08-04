"use client";

import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  WORKFLOW_ANIMATION_BODY,
  WORKFLOW_ANIMATION_INNER,
  WORKFLOW_ANIMATION_SHELL,
} from "@/app/lib/marketing-theme";

const PHASES = [
  "form",
  "envoi",
  "calendrier",
  "switch",
  "invite",
  "docs",
  "compte",
  "pilotage",
] as const;
type Phase = (typeof PHASES)[number];
const PHASE_MS = 1900;

const WEEK = ["L", "M", "M", "J", "V"];
const CAL_MARK = 1; // mardi

const PHASE_LABEL: Record<Phase, string> = {
  form: "Absence",
  envoi: "Envoi",
  calendrier: "Agenda",
  switch: "RH",
  invite: "Invitation",
  docs: "Docs",
  compte: "Compte",
  pilotage: "Pilotage",
};

const TOAST: Record<Phase, string> = {
  form: "Déclaration d'absence…",
  envoi: "Demande transmise",
  calendrier: "Visibilité équipe",
  switch: "Onboarding nouveau salarié",
  invite: "Invitation envoyée",
  docs: "Documents à fournir",
  compte: "Compte créé",
  pilotage: "RH à jour",
};

export default function WorkflowAbsencesAnimation() {
  const [phase, setPhase] = useState<Phase>("form");
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => {
      setPhase((p) => PHASES[(PHASES.indexOf(p) + 1) % PHASES.length]);
      setTick((t) => t + 1);
    }, PHASE_MS);
    return () => clearInterval(id);
  }, []);

  const absMode = phase === "form" || phase === "envoi" || phase === "calendrier";
  const onboarding =
    phase === "switch" ||
    phase === "invite" ||
    phase === "docs" ||
    phase === "compte" ||
    phase === "pilotage";

  return (
    <div
      className={`${WORKFLOW_ANIMATION_SHELL} border border-rose-400/20 bg-gradient-to-br from-[#5C2D3A] via-[#6B3A4A] to-[#452530] shadow-rose-900/20`}
    >
      <div className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-rose-400/15 blur-2xl" />
      <div className={`${WORKFLOW_ANIMATION_INNER} bg-[#2A1520]/50`}>
        <div className="mb-2 flex shrink-0 items-center gap-2 border-b border-white/10 pb-2.5">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-rose-400/80" />
          <span className="ml-2 text-xs font-semibold text-rose-100/80">Ressources humaines</span>
          <motion.span
            key={tick}
            initial={{ opacity: 0, scale: 0.85 }}
            animate={{ opacity: 1, scale: 1 }}
            className="ml-auto rounded-full bg-rose-400/20 px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-rose-200"
          >
            {PHASE_LABEL[phase]}
          </motion.span>
        </div>

        {/* Mini onglets Absences / Onboarding */}
        <div className="mb-2 flex gap-1">
          {[
            { id: "abs", label: "Absences", on: absMode },
            { id: "onb", label: "Onboarding", on: onboarding },
          ].map((t) => (
            <div
              key={t.id}
              className={`rounded-lg px-2.5 py-1 text-[10px] font-bold uppercase tracking-wide transition-colors ${
                t.on
                  ? "bg-rose-400/25 text-rose-100 ring-1 ring-rose-300/40"
                  : "bg-white/5 text-white/35"
              }`}
            >
              {t.label}
            </div>
          ))}
        </div>

        <div className={`${WORKFLOW_ANIMATION_BODY} grid gap-2.5 sm:grid-cols-2`}>
          <div className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2.5">
            <p className="text-[10px] font-bold uppercase tracking-wider text-rose-200/60">
              {absMode ? "Se déclarer" : "Arrivée"}
            </p>
            <div className="relative mt-1.5 min-h-0 flex-1">
              <AnimatePresence mode="wait">
                {absMode ? (
                  <motion.div
                    key="abs"
                    initial={{ opacity: 0, x: -8 }}
                    animate={{ opacity: 1, x: 0 }}
                    exit={{ opacity: 0, x: -8 }}
                    className="space-y-1.5"
                  >
                    <div className="rounded-lg bg-white/10 px-2.5 py-1.5">
                      <p className="text-[10px] text-rose-200/50">Période</p>
                      <p className="text-xs font-bold text-white">Mardi 14 · 8h30 → 12h30</p>
                    </div>
                    <div className="rounded-lg bg-white/10 px-2.5 py-1.5">
                      <p className="text-[10px] text-rose-200/50">Motif</p>
                      <p className="text-xs text-white/90">Rendez-vous médical</p>
                    </div>

                    {phase === "form" ? (
                      <motion.div
                        animate={{ scale: [1, 1.02, 1] }}
                        transition={{ repeat: Infinity, duration: 1.6 }}
                        className="rounded-lg bg-rose-500/35 py-2 text-center text-[11px] font-black text-white"
                      >
                        Envoyer la déclaration
                      </motion.div>
                    ) : phase === "envoi" ? (
                      <p className="text-center text-[11px] font-bold text-amber-200">
                        Transmission…
                      </p>
                    ) : (
                      <div className="space-y-1.5">
                        <p className="text-center text-[11px] font-bold text-[#4ADE80]">
                          ✓ Sur le calendrier
                        </p>
                        <div className="flex justify-between gap-0.5 rounded-lg bg-white/5 p-1.5">
                          {WEEK.map((d, i) => (
                            <div
                              key={`${d}-${i}`}
                              className={`flex h-7 w-7 flex-col items-center justify-center rounded-md text-[9px] font-bold ${
                                i === CAL_MARK
                                  ? "bg-rose-400/50 text-white ring-1 ring-rose-300/50"
                                  : "bg-white/5 text-rose-100/40"
                              }`}
                            >
                              {d}
                              {i === CAL_MARK ? (
                                <span className="text-[7px] text-rose-100">½j</span>
                              ) : null}
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </motion.div>
                ) : (
                  <motion.div
                    key="onb"
                    initial={{ opacity: 0, x: 8 }}
                    animate={{ opacity: 1, x: 0 }}
                    exit={{ opacity: 0, x: 8 }}
                    className="space-y-1.5"
                  >
                    {phase === "switch" ? (
                      <motion.p
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        className="py-4 text-center text-[11px] text-rose-100/60"
                      >
                        Basculement vers l&apos;onboarding…
                      </motion.p>
                    ) : (
                      <>
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
                            <span
                              className={
                                phase !== "switch" ? "text-[#4ADE80]" : "text-white/30"
                              }
                            >
                              {phase !== "switch" ? "✓" : "—"}
                            </span>
                          </li>
                          <li className="flex justify-between rounded bg-white/5 px-2 py-1">
                            <span>Documents à fournir</span>
                            <span
                              className={
                                phase === "docs"
                                  ? "text-amber-200"
                                  : phase === "compte" || phase === "pilotage"
                                    ? "text-[#4ADE80]"
                                    : "text-white/30"
                              }
                            >
                              {phase === "docs"
                                ? "…"
                                : phase === "compte" || phase === "pilotage"
                                  ? "✓"
                                  : "—"}
                            </span>
                          </li>
                          <li className="flex justify-between rounded bg-white/5 px-2 py-1">
                            <span>Compte créé</span>
                            <span
                              className={
                                phase === "compte" || phase === "pilotage"
                                  ? "text-[#4ADE80]"
                                  : "text-white/30"
                              }
                            >
                              {phase === "compte" || phase === "pilotage" ? "✓" : "—"}
                            </span>
                          </li>
                        </ul>
                      </>
                    )}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>

          <div className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2.5">
            <p className="text-[10px] font-bold uppercase tracking-wider text-rose-200/60">
              Pilotage RH
            </p>
            <ul className="mt-1.5 flex-1 space-y-1.5">
              {[
                {
                  label: "Absences du jour",
                  value:
                    phase === "calendrier" || phase === "envoi"
                      ? "3"
                      : phase === "form"
                        ? "2"
                        : "3",
                },
                {
                  label: "À valider",
                  value: phase === "form" || phase === "envoi" ? "1" : "0",
                },
                {
                  label: "Onboardings ouverts",
                  value:
                    phase === "invite" || phase === "docs" || phase === "compte"
                      ? "1"
                      : phase === "pilotage"
                        ? "0"
                        : "0",
                },
                {
                  label: "Dossiers à jour",
                  value: phase === "pilotage" ? "Oui" : onboarding ? "…" : "—",
                },
              ].map((row, i) => (
                <motion.li
                  key={row.label}
                  initial={false}
                  animate={{
                    opacity: phase === "pilotage" ? 1 : 0.85,
                    scale: phase === "pilotage" && i === 3 ? 1.02 : 1,
                  }}
                  className={`flex items-center justify-between rounded-lg px-2.5 py-1.5 ${
                    phase === "pilotage" && i === 3
                      ? "bg-emerald-500/20 ring-1 ring-emerald-400/30"
                      : "bg-white/5"
                  }`}
                >
                  <span className="text-[11px] text-rose-100/80">{row.label}</span>
                  <span className="text-xs font-black text-white">{row.value}</span>
                </motion.li>
              ))}
            </ul>
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
                phase === "pilotage"
                  ? "bg-emerald-400/20 text-[#4ADE80] ring-1 ring-emerald-300/40"
                  : "bg-white/10 text-rose-100/85"
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
                  phase === p ? "w-5 bg-rose-300" : "w-1.5 bg-white/20"
                }`}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
