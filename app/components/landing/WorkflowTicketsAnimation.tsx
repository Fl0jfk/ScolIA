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
  "pj",
  "envoi",
  "route",
  "kanban",
  "prise",
  "termine",
] as const;
type Phase = (typeof PHASES)[number];
const PHASE_MS = 1900;

const COLUMNS = [
  { id: "new", label: "Nouvelles" },
  { id: "progress", label: "En cours" },
  { id: "done", label: "Terminées" },
] as const;

const PHASE_LABEL: Record<Phase, string> = {
  form: "Création",
  pj: "Pièce jointe",
  envoi: "Envoi",
  route: "Routage",
  kanban: "Kanban",
  prise: "Prise en charge",
  termine: "Clôturé",
};

const TOAST: Record<Phase, string> = {
  form: "Nouvelle demande…",
  pj: "Photo jointe",
  envoi: "Ticket envoyé",
  route: "Vers Maintenance",
  kanban: "Dans la bonne corbeille",
  prise: "Pris en charge · Marc T.",
  termine: "Ticket suivi · plus de mail perdu",
};

export default function WorkflowTicketsAnimation() {
  const [phase, setPhase] = useState<Phase>("form");
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => {
      setPhase((p) => PHASES[(PHASES.indexOf(p) + 1) % PHASES.length]);
      setTick((t) => t + 1);
    }, PHASE_MS);
    return () => clearInterval(id);
  }, []);

  const cardCol =
    phase === "termine"
      ? "done"
      : phase === "prise" || phase === "kanban"
        ? "progress"
        : phase === "route" || phase === "envoi"
          ? "new"
          : null;

  const showBoard =
    phase === "route" ||
    phase === "kanban" ||
    phase === "prise" ||
    phase === "termine";

  return (
    <div
      className={`${WORKFLOW_ANIMATION_SHELL} border border-amber-400/25 bg-gradient-to-br from-[#5C3D1E] via-[#8B5E34] to-[#3D2A14] shadow-amber-900/25`}
    >
      <div className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-amber-400/20 blur-2xl" />
      <div className={`${WORKFLOW_ANIMATION_INNER} bg-[#2A1A0C]/50`}>
        <div className="mb-2 flex shrink-0 items-center gap-2 border-b border-white/10 pb-2.5">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-orange-400/80" />
          <span className="ml-2 text-xs font-semibold text-amber-100/80">
            Ticketing · Demandes
          </span>
          <motion.span
            key={tick}
            initial={{ opacity: 0, scale: 0.85 }}
            animate={{ opacity: 1, scale: 1 }}
            className="ml-auto rounded-full bg-amber-400/20 px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-amber-200"
          >
            {PHASE_LABEL[phase]}
          </motion.span>
        </div>

        <div className={`${WORKFLOW_ANIMATION_BODY} grid gap-2.5 sm:grid-cols-2`}>
          {/* Formulaire / ticket */}
          <div className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2.5">
            <p className="text-[10px] font-bold uppercase tracking-wider text-amber-200/60">
              {showBoard && phase !== "route" ? "Ticket #4821" : "Faire une demande"}
            </p>
            <div className="relative mt-1.5 min-h-0 flex-1">
              <AnimatePresence mode="wait">
                {phase === "form" || phase === "pj" ? (
                  <motion.div
                    key="form"
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    <div className="rounded-lg bg-white/10 px-2.5 py-1.5">
                      <p className="text-[10px] text-amber-200/50">Service</p>
                      <p className="text-xs font-bold text-white">Maintenance</p>
                    </div>
                    <div className="rounded-lg bg-white/10 px-2.5 py-1.5">
                      <p className="text-[10px] text-amber-200/50">Objet</p>
                      <p className="text-xs font-bold text-white">Volet cassé — salle 12</p>
                    </div>
                    {phase === "pj" ? (
                      <motion.div
                        initial={{ opacity: 0, y: -8 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="flex items-center gap-2 rounded-lg bg-amber-500/20 px-2.5 py-1.5 ring-1 ring-amber-400/30"
                      >
                        <span className="flex h-7 w-7 items-center justify-center rounded-md bg-white/15 text-[10px]">
                          📷
                        </span>
                        <div className="min-w-0">
                          <p className="truncate text-[11px] font-bold text-white">
                            photo_volet.jpg
                          </p>
                          <p className="text-[9px] text-amber-200/55">Joindre une photo</p>
                        </div>
                      </motion.div>
                    ) : (
                      <div className="rounded-lg border border-dashed border-amber-400/30 py-2 text-center text-[10px] text-amber-200/45">
                        + Pièce jointe
                      </div>
                    )}
                  </motion.div>
                ) : phase === "envoi" ? (
                  <motion.div
                    key="send"
                    initial={{ opacity: 0, scale: 0.96 }}
                    animate={{ opacity: 1, scale: 1 }}
                    exit={{ opacity: 0 }}
                    className="flex h-full flex-col items-center justify-center gap-2"
                  >
                    <motion.div
                      animate={{ y: [0, -4, 0] }}
                      transition={{ repeat: Infinity, duration: 1.1 }}
                      className="rounded-xl bg-amber-500/30 px-4 py-3 text-center ring-1 ring-amber-400/40"
                    >
                      <p className="text-xs font-black text-white">Envoi du ticket…</p>
                      <p className="mt-0.5 text-[10px] text-amber-100/70">Réf. #4821</p>
                    </motion.div>
                  </motion.div>
                ) : (
                  <motion.div
                    key="detail"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    <div className="rounded-lg bg-white/10 px-2.5 py-1.5">
                      <p className="text-[10px] text-amber-200/50">Demandeur</p>
                      <p className="text-xs font-bold text-white">Mme Lefebvre · Vie scolaire</p>
                    </div>
                    <div className="rounded-lg bg-white/10 px-2.5 py-1.5">
                      <p className="text-[10px] text-amber-200/50">Statut</p>
                      <p className="text-xs font-bold text-white">
                        {phase === "termine"
                          ? "Terminé"
                          : phase === "prise"
                            ? "En cours · Marc T."
                            : phase === "kanban"
                              ? "Nouvelle · Maintenance"
                              : "Routage…"}
                      </p>
                    </div>
                    {(phase === "prise" || phase === "termine") && (
                      <motion.p
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        className="text-center text-[10px] font-bold text-[#4ADE80]"
                      >
                        {phase === "termine" ? "✓ Intervention clôturée" : "Pris en charge"}
                      </motion.p>
                    )}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>

          {/* Kanban / corbeille */}
          <div className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2.5">
            <p className="text-[10px] font-bold uppercase tracking-wider text-amber-200/60">
              Corbeille Maintenance
            </p>
            <div className="mt-1.5 grid min-h-0 flex-1 grid-cols-3 gap-1">
              {COLUMNS.map((col) => {
                const hasCard = cardCol === col.id;
                const dim = !showBoard;
                return (
                  <div
                    key={col.id}
                    className="flex min-h-0 flex-col rounded-lg bg-black/20 p-1"
                  >
                    <p className="mb-1 truncate text-center text-[8px] font-bold uppercase tracking-wide text-amber-200/50">
                      {col.label}
                    </p>
                    <div className="min-h-0 flex-1">
                      <AnimatePresence>
                        {hasCard && !dim ? (
                          <motion.div
                            layout
                            key="ticket-card"
                            layoutId="ticket-card"
                            initial={{ opacity: 0, scale: 0.9 }}
                            animate={{ opacity: 1, scale: 1 }}
                            exit={{ opacity: 0, scale: 0.9 }}
                            className={`rounded-md px-1.5 py-1.5 ring-1 ${
                              col.id === "done"
                                ? "bg-emerald-500/25 ring-emerald-400/40"
                                : col.id === "progress"
                                  ? "bg-sky-500/25 ring-sky-400/40"
                                  : "bg-amber-500/25 ring-amber-400/40"
                            }`}
                          >
                            <p className="text-[8px] font-black text-white">#4821</p>
                            <p className="mt-0.5 line-clamp-2 text-[8px] leading-tight text-white/80">
                              Volet salle 12
                            </p>
                            {col.id === "progress" || col.id === "done" ? (
                              <p className="mt-0.5 text-[7px] font-bold text-white/60">Marc T.</p>
                            ) : null}
                          </motion.div>
                        ) : null}
                      </AnimatePresence>
                      {dim && col.id === "new" ? (
                        <p className="px-0.5 pt-2 text-center text-[8px] text-amber-200/30">
                          En attente…
                        </p>
                      ) : null}
                    </div>
                  </div>
                );
              })}
            </div>
            {phase === "route" ? (
              <motion.p
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                className="mt-1 text-center text-[9px] font-bold text-amber-200"
              >
                Routage → Maintenance ✓
              </motion.p>
            ) : null}
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
                phase === "termine"
                  ? "bg-emerald-400/20 text-[#4ADE80] ring-1 ring-emerald-300/40"
                  : "bg-white/10 text-amber-100/85"
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
                  phase === p ? "w-5 bg-amber-400" : "w-1.5 bg-white/20"
                }`}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
