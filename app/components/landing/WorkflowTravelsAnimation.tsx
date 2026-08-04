"use client";

import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  WORKFLOW_ANIMATION_BODY,
  WORKFLOW_ANIMATION_INNER,
  WORKFLOW_ANIMATION_SHELL,
} from "@/app/lib/marketing-theme";

const PHASES = [
  "dossier",
  "demande",
  "devis",
  "ocr",
  "signature",
  "compta",
  "done",
] as const;
type Phase = (typeof PHASES)[number];
const PHASE_MS = 2000;

const TABS = [
  { id: "overview", label: "Vue" },
  { id: "transport", label: "Transport" },
  { id: "compta", label: "Compta" },
] as const;

const STEPS = [
  { label: "Dossier créé", at: "dossier" as Phase },
  { label: "Demande de devis", at: "demande" as Phase },
  { label: "PDF reçu", at: "devis" as Phase },
  { label: "Montant OCR", at: "ocr" as Phase },
  { label: "Signature auto", at: "signature" as Phase },
  { label: "Fiche compta", at: "compta" as Phase },
];

const PHASE_LABEL: Record<Phase, string> = {
  dossier: "Création",
  demande: "Devis",
  devis: "Réception",
  ocr: "OCR",
  signature: "Signature",
  compta: "Compta",
  done: "Validé",
};

const TOAST: Record<Phase, string> = {
  dossier: "Nouveau dossier sortie…",
  demande: "Demande envoyée au transporteur",
  devis: "devis_dupont.pdf reçu",
  ocr: "OCR → 1 250 € TTC",
  signature: "Devis signé automatiquement",
  compta: "Budget & coût élève à jour",
  done: "Validé · communication unifiée",
};

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
    phase === "compta" || phase === "done"
      ? "compta"
      : phase === "demande" || phase === "devis" || phase === "ocr" || phase === "signature"
        ? "transport"
        : "overview";

  const phaseIdx = PHASES.indexOf(phase);

  return (
    <div
      className={`${WORKFLOW_ANIMATION_SHELL} border border-indigo-400/25 bg-gradient-to-br from-[#1E3A5F] via-[#234B73] to-[#1A3348] shadow-indigo-900/25`}
    >
      <div className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-sky-400/20 blur-2xl" />
      <div className={`${WORKFLOW_ANIMATION_INNER} bg-[#0C1A28]/50`}>
        <div className="mb-2 flex shrink-0 items-center gap-2 border-b border-white/10 pb-2.5">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-sky-400/80" />
          <span className="ml-2 text-xs font-semibold text-sky-100/80">Sortie scolaire</span>
          <motion.span
            key={tick}
            initial={{ opacity: 0, scale: 0.85 }}
            animate={{ opacity: 1, scale: 1 }}
            className="ml-auto rounded-full bg-sky-400/20 px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-sky-200"
          >
            {PHASE_LABEL[phase]}
          </motion.span>
        </div>

        <div className="mb-2 flex gap-1">
          {TABS.map((t) => (
            <motion.div
              key={t.id}
              animate={{
                backgroundColor:
                  activeTab === t.id ? "rgba(56,189,248,0.25)" : "rgba(255,255,255,0.05)",
              }}
              className={`rounded-lg px-2.5 py-1 text-[10px] font-bold uppercase tracking-wide ${
                activeTab === t.id
                  ? "text-sky-100 ring-1 ring-sky-300/40"
                  : "text-white/40"
              }`}
            >
              {t.label}
            </motion.div>
          ))}
        </div>

        <div className={`${WORKFLOW_ANIMATION_BODY} grid gap-2.5 sm:grid-cols-2`}>
          <div className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2.5">
            <div className="flex items-start gap-2.5">
              <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-amber-400/20 text-base">
                🚌
              </div>
              <div className="min-w-0">
                <p className="truncate text-sm font-bold text-white">Musée — 6A</p>
                <p className="text-[11px] text-sky-100/55">45 élèves · Bus · 12 mars</p>
              </div>
            </div>

            <div className="relative mt-2.5 min-h-0 flex-1">
              <AnimatePresence mode="wait">
                {phase === "dossier" ? (
                  <motion.div
                    key="dossier"
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                    className="space-y-2"
                  >
                    <div className="rounded-lg bg-white/10 px-2.5 py-2">
                      <p className="text-[10px] text-sky-200/50">Destination</p>
                      <p className="text-xs font-bold text-white">Musée des Arts et Métiers</p>
                    </div>
                    <p className="text-[11px] text-sky-100/50">Circuit direction lancé…</p>
                  </motion.div>
                ) : phase === "demande" ? (
                  <motion.div
                    key="demande"
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                    className="space-y-2"
                  >
                    <div className="rounded-lg bg-sky-500/20 px-2.5 py-2 ring-1 ring-sky-400/30">
                      <p className="text-[10px] font-bold text-sky-100">→ Demande devis</p>
                      <p className="mt-0.5 text-[10px] text-sky-100/60">
                        Transports Dupont · A/R Paris
                      </p>
                    </div>
                    <motion.div
                      animate={{ opacity: [0.4, 1, 0.4] }}
                      transition={{ repeat: Infinity, duration: 1.4 }}
                      className="text-center text-[10px] font-bold text-sky-200/70"
                    >
                      En attente de réponse…
                    </motion.div>
                  </motion.div>
                ) : phase === "devis" || phase === "ocr" ? (
                  <motion.div
                    key="devis"
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    <motion.div
                      initial={{ y: -16, opacity: 0 }}
                      animate={{ y: 0, opacity: 1 }}
                      className="flex items-center gap-2 rounded-lg bg-white/10 px-2.5 py-1.5"
                    >
                      <span className="rounded bg-red-500/90 px-1 text-[8px] font-black text-white">
                        PDF
                      </span>
                      <span className="truncate text-[11px] font-semibold text-white">
                        devis_dupont.pdf
                      </span>
                    </motion.div>
                    {phase === "ocr" ? (
                      <div className="space-y-1">
                        <div className="h-1.5 overflow-hidden rounded-full bg-white/10">
                          <motion.div
                            key={`ocr-${tick}`}
                            className="h-full rounded-full bg-gradient-to-r from-sky-400 to-amber-300"
                            initial={{ width: "0%" }}
                            animate={{ width: "100%" }}
                            transition={{ duration: 1.4 }}
                          />
                        </div>
                        <p className="text-[10px] font-bold text-amber-200">IA OCR → 1 250 € TTC</p>
                      </div>
                    ) : (
                      <p className="text-[10px] text-sky-100/55">Pièce jointe reçue</p>
                    )}
                  </motion.div>
                ) : phase === "signature" ? (
                  <motion.div
                    key="sig"
                    initial={{ opacity: 0, scale: 0.96 }}
                    animate={{ opacity: 1, scale: 1 }}
                    exit={{ opacity: 0 }}
                    className="rounded-xl bg-emerald-500/20 p-2.5 ring-1 ring-emerald-400/40"
                  >
                    <p className="text-[11px] font-black text-[#4ADE80]">
                      ✓ Signature automatique
                    </p>
                    <p className="mt-1 text-[10px] text-emerald-100/70">
                      Bon de commande envoyé au transporteur
                    </p>
                    <div className="relative mt-2 h-10 overflow-hidden rounded-lg border border-dashed border-emerald-300/40 bg-white/5 px-2">
                      <motion.svg
                        viewBox="0 0 160 36"
                        className="absolute inset-x-2 top-1 h-8 w-[90%]"
                        initial={{ pathLength: 0, opacity: 0 }}
                        animate={{ pathLength: 1, opacity: 1 }}
                      >
                        <motion.path
                          d="M4 24 C 28 8, 42 30, 58 18 S 90 6, 110 22 S 140 28, 156 14"
                          fill="none"
                          stroke="rgba(167,243,208,0.9)"
                          strokeWidth="2.2"
                          strokeLinecap="round"
                          initial={{ pathLength: 0 }}
                          animate={{ pathLength: 1 }}
                          transition={{ duration: 1.2, ease: "easeInOut" }}
                        />
                      </motion.svg>
                      <motion.span
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        transition={{ delay: 0.9 }}
                        className="absolute bottom-0.5 right-2 rounded bg-emerald-500/40 px-1.5 py-0.5 text-[8px] font-black uppercase tracking-wide text-emerald-50"
                      >
                        Tampon Dir.
                      </motion.span>
                    </div>
                  </motion.div>
                ) : (
                  <motion.div
                    key="compta"
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    <p className="text-[10px] font-bold uppercase text-sky-200/60">Fiche budget</p>
                    <div className="space-y-1 rounded-lg bg-white/10 px-2.5 py-2 text-[11px] text-white">
                      <div className="flex justify-between">
                        <span className="text-sky-100/60">Transport</span>
                        <span className="font-bold">1 250 €</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-sky-100/60">Restauration</span>
                        <span className="font-bold">380 €</span>
                      </div>
                      <div className="mt-1 flex justify-between border-t border-white/10 pt-1">
                        <span className="text-sky-100/60">Coût / élève</span>
                        <span className="font-black text-[#4ADE80]">36,20 €</span>
                      </div>
                    </div>
                    {phase === "done" ? (
                      <p className="text-center text-[10px] font-black text-[#4ADE80]">
                        Compta synchronisée ✓
                      </p>
                    ) : null}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </div>

          <div className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2.5">
            <p className="text-[10px] font-bold uppercase tracking-wider text-sky-200/60">Parcours</p>
            <ul className="mt-1.5 flex-1 space-y-1 overflow-y-auto">
              {STEPS.map((item) => {
                const lit = phaseIdx >= PHASES.indexOf(item.at);
                return (
                  <li
                    key={item.label}
                    className={`flex items-center gap-2 rounded-lg px-2 py-1 transition-all ${
                      lit ? "bg-emerald-500/15 ring-1 ring-emerald-400/30" : "bg-white/5 opacity-35"
                    }`}
                  >
                    <span className="flex-1 text-[10px] font-semibold text-white">{item.label}</span>
                    {lit ? <span className="text-[10px] font-black text-[#4ADE80]">✓</span> : null}
                  </li>
                );
              })}
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
                phase === "done"
                  ? "bg-emerald-400/20 text-[#4ADE80] ring-1 ring-emerald-300/40"
                  : "bg-white/10 text-sky-100/85"
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
                  phase === p ? "w-5 bg-sky-400" : "w-1.5 bg-white/20"
                }`}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
