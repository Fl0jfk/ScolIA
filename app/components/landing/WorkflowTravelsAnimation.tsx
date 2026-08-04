"use client";

import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  WORKFLOW_ANIMATION_BODY,
  WORKFLOW_ANIMATION_INNER,
  WORKFLOW_ANIMATION_SHELL,
} from "@/app/lib/marketing-theme";

const PHASES = ["dossier", "devis", "signature", "compta"] as const;
type Phase = (typeof PHASES)[number];
const PHASE_MS = 2400;

const TABS = [
  { id: "overview", label: "Vue" },
  { id: "transport", label: "Transport" },
  { id: "compta", label: "Compta" },
] as const;

export default function WorkflowTravelsAnimation() {
  const [phase, setPhase] = useState<Phase>("dossier");
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => {
      setPhase((p) => PHASES[(PHASES.indexOf(p) + 1) % PHASES.length]);
      setTick((t) => t + 1);
    }, PHASE_MS);
    return () => clearInterval(id);
  }, []);

  const activeTab =
    phase === "compta" ? "compta" : phase === "devis" || phase === "signature" ? "transport" : "overview";

  return (
    <div
      className={`${WORKFLOW_ANIMATION_SHELL} border border-indigo-400/25 bg-gradient-to-br from-[#1E3A5F] via-[#234B73] to-[#1A3348] shadow-indigo-900/25`}
    >
      <div className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-sky-400/20 blur-2xl" />
      <div className={`${WORKFLOW_ANIMATION_INNER} bg-[#0C1A28]/50`}>
        <div className="mb-3 flex shrink-0 items-center gap-2 border-b border-white/10 pb-3">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-sky-400/80" />
          <span className="ml-2 text-xs font-semibold text-sky-100/80">Sortie scolaire</span>
          <motion.span
            key={tick}
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            className="ml-auto rounded-full bg-sky-400/20 px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-sky-200"
          >
            {phase === "signature" ? "Signature auto" : phase === "compta" ? "Onglet compta" : "Workflow"}
          </motion.span>
        </div>

        <div className="mb-3 flex gap-1">
          {TABS.map((t) => (
            <div
              key={t.id}
              className={`rounded-lg px-2.5 py-1 text-[10px] font-bold uppercase tracking-wide transition-colors ${
                activeTab === t.id
                  ? "bg-sky-400/25 text-sky-100 ring-1 ring-sky-300/40"
                  : "bg-white/5 text-white/40"
              }`}
            >
              {t.label}
            </div>
          ))}
        </div>

        <div className={`${WORKFLOW_ANIMATION_BODY} grid gap-3 sm:grid-cols-2`}>
          <div className="flex flex-col rounded-2xl border border-white/10 bg-white/5 p-3">
            <div className="flex items-start gap-2.5">
              <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-amber-400/20 text-lg">
                🚌
              </div>
              <div className="min-w-0">
                <p className="truncate text-sm font-bold text-white">Musée — 6A</p>
                <p className="text-[11px] text-sky-100/55">45 élèves · Bus · 12 mars</p>
              </div>
            </div>
            <div className="relative mt-3 min-h-[5.5rem] flex-1">
              <AnimatePresence mode="wait">
                {phase === "dossier" ? (
                  <motion.p
                    key="d"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="text-[11px] text-sky-100/50"
                  >
                    Circuit direction lancé…
                  </motion.p>
                ) : phase === "devis" ? (
                  <motion.div
                    key="dv"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    <div className="flex items-center gap-2 rounded-lg bg-white/10 px-2.5 py-1.5">
                      <span className="rounded bg-red-500/90 px-1 text-[8px] font-black text-white">PDF</span>
                      <span className="truncate text-[11px] font-semibold text-white">
                        devis_dupont.pdf
                      </span>
                    </div>
                    <p className="text-[10px] font-bold text-amber-200">IA OCR → 1 250 € TTC</p>
                  </motion.div>
                ) : phase === "signature" ? (
                  <motion.div
                    key="sg"
                    initial={{ opacity: 0, scale: 0.96 }}
                    animate={{ opacity: 1, scale: 1 }}
                    exit={{ opacity: 0 }}
                    className="rounded-xl bg-emerald-500/20 p-3 ring-1 ring-emerald-400/40"
                  >
                    <p className="text-[11px] font-black text-[#4ADE80]">✓ Devis signé automatiquement</p>
                    <p className="mt-1 text-[10px] text-emerald-100/70">
                      Bon de commande envoyé au transporteur
                    </p>
                    <div className="mt-2 h-8 rounded-lg border border-dashed border-emerald-300/40 bg-white/5 px-2 py-1 font-serif text-sm italic text-emerald-100/80">
                      Dir. Collège — signature
                    </div>
                  </motion.div>
                ) : (
                  <motion.div
                    key="cp"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    <p className="text-[10px] font-bold uppercase text-sky-200/60">Fiche budget</p>
                    <div className="rounded-lg bg-white/10 px-2.5 py-2 text-[11px] text-white">
                      Transport 1 250 € · Restau 380 €
                    </div>
                    <p className="text-[10px] font-bold text-[#4ADE80]">Compta : coût / élève à jour</p>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>

          <div className="flex flex-col rounded-2xl border border-white/10 bg-white/5 p-3">
            <p className="text-[10px] font-bold uppercase tracking-wider text-sky-200/60">Parcours</p>
            <ul className="mt-2 flex-1 space-y-1.5">
              {[
                { label: "Validation pédagogique", at: "dossier" },
                { label: "Devis transport reçu", at: "devis" },
                { label: "Signature devis auto", at: "signature" },
                { label: "Onglet compta renseigné", at: "compta" },
              ].map((item) => {
                const lit = PHASES.indexOf(phase) >= PHASES.indexOf(item.at as Phase);
                return (
                  <li
                    key={item.label}
                    className={`flex items-center gap-2 rounded-lg px-2 py-1.5 transition-all ${
                      lit ? "bg-emerald-500/15 ring-1 ring-emerald-400/30" : "bg-white/5 opacity-35"
                    }`}
                  >
                    <span className="flex-1 text-[11px] font-semibold text-white">{item.label}</span>
                    {lit ? <span className="text-[10px] font-black text-[#4ADE80]">✓</span> : null}
                  </li>
                );
              })}
            </ul>
          </div>
        </div>

        <div className="mt-3 flex justify-center gap-1.5">
          {PHASES.map((p) => (
            <span
              key={p}
              className={`h-1.5 rounded-full transition-all ${
                phase === p ? "w-6 bg-sky-400" : "w-1.5 bg-white/20"
              }`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
