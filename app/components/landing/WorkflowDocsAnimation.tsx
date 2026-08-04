"use client";

import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  WORKFLOW_ANIMATION_BODY,
  WORKFLOW_ANIMATION_INNER,
  WORKFLOW_ANIMATION_SHELL,
} from "@/app/lib/marketing-theme";

const PHASES = ["drop", "scan", "tags", "split", "fly", "folders", "done"] as const;
type Phase = (typeof PHASES)[number];
const PHASE_MS = 2000;

const BATCH = [
  { name: "bulletins_T2.pdf", meta: "Lot · 28 pages", delay: 0 },
  { name: "releve_BAC.pdf", meta: "Relevé bac", delay: 0.18 },
];

const OCR_TAGS = [
  { label: "OCR", color: "bg-amber-400/25 text-amber-100 ring-amber-300/40" },
  { label: "Élève", color: "bg-sky-400/25 text-sky-100 ring-sky-300/40" },
  { label: "Type", color: "bg-emerald-400/25 text-emerald-100 ring-emerald-300/40" },
];

const CARDS = [
  { name: "Dupont Marie", klass: "6A", file: "bulletin_T2.pdf", folder: "Dupont" },
  { name: "Martin Lucas", klass: "5B", file: "bulletin_T2.pdf", folder: "Martin" },
  { name: "Bernard Léa", klass: "Tle", file: "releve_bac.pdf", folder: "Bac / Tle" },
];

const FOLDERS = [
  { name: "Dupont Marie — 6A", path: "OneDrive / Élèves / Dupont" },
  { name: "Martin Lucas — 5B", path: "OneDrive / Élèves / Martin" },
  { name: "Bernard Léa — Tle", path: "OneDrive / Bac / Tle" },
];

const PHASE_LABEL: Record<Phase, string> = {
  drop: "Dépôt",
  scan: "OCR",
  tags: "Lecture",
  split: "Découpe",
  fly: "Classement",
  folders: "OneDrive",
  done: "Terminé",
};

const TOAST: Record<Phase, string> = {
  drop: "Déposez un lot PDF…",
  scan: "OCR Mistral en cours…",
  tags: "Élève · type · période détectés",
  split: "Pages → fiches élèves",
  fly: "Envoi vers OneDrive…",
  folders: "Dossiers mis à jour",
  done: "Classé · moins de papier",
};

export default function WorkflowDocsAnimation() {
  const [phase, setPhase] = useState<Phase>("drop");
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => {
      setPhase((p) => PHASES[(PHASES.indexOf(p) + 1) % PHASES.length]);
      setTick((t) => t + 1);
    }, PHASE_MS);
    return () => clearInterval(id);
  }, []);

  const showBatch = phase === "drop" || phase === "scan" || phase === "tags";
  const showScan = phase === "scan" || phase === "tags";
  const showTags = phase === "tags" || phase === "split";
  const showCards = phase === "split" || phase === "fly";
  const showFolders = phase === "fly" || phase === "folders" || phase === "done";
  const foldersDone = phase === "folders" || phase === "done";

  return (
    <div
      className={`${WORKFLOW_ANIMATION_SHELL} border border-emerald-400/30 bg-gradient-to-br from-[#1E4A32] via-[#2F6B4A] to-[#1A3D2B] shadow-emerald-900/30`}
    >
      <div className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-[#4ADE80]/20 blur-2xl" />
      <div className={`${WORKFLOW_ANIMATION_INNER} bg-[#0F2318]/40`}>
        <div className="mb-2 flex shrink-0 items-center gap-2 border-b border-white/10 pb-2.5">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-emerald-400/80" />
          <span className="ml-2 text-xs font-semibold text-emerald-100/80">Documents élèves · IA</span>
          <motion.span
            key={tick}
            initial={{ opacity: 0, scale: 0.85 }}
            animate={{ opacity: 1, scale: 1 }}
            className="ml-auto rounded-full bg-[#F59E0B]/20 px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-amber-200"
          >
            {PHASE_LABEL[phase]}
          </motion.span>
        </div>

        <div className={`${WORKFLOW_ANIMATION_BODY} relative grid gap-2.5 sm:grid-cols-[1fr_1fr]`}>
          {/* Colonne gauche : drop / OCR / split */}
          <div className="relative flex min-h-0 flex-col overflow-hidden rounded-2xl border-2 border-dashed border-emerald-400/35 bg-white/5 p-2.5">
            <p className="shrink-0 text-[10px] font-bold uppercase tracking-wider text-emerald-200/70">
              {showCards ? "Fiches élèves" : "Zone de dépôt"}
            </p>

            <div className="relative mt-1.5 min-h-0 flex-1">
              <AnimatePresence mode="wait">
                {showBatch && !showCards ? (
                  <motion.div
                    key="batch"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0, y: -8 }}
                    className="space-y-1.5"
                  >
                    {BATCH.map((f) => (
                      <motion.div
                        key={f.name}
                        initial={{ opacity: 0, y: -28, rotate: -4 }}
                        animate={{ opacity: 1, y: 0, rotate: 0 }}
                        transition={{ delay: f.delay, type: "spring", stiffness: 280, damping: 22 }}
                        className="flex items-center gap-2 rounded-xl bg-white/10 px-2.5 py-2"
                      >
                        <div className="flex h-8 w-7 items-center justify-center rounded-md bg-red-500/90 text-[9px] font-black text-white">
                          PDF
                        </div>
                        <div className="min-w-0">
                          <p className="truncate text-xs font-bold text-white">{f.name}</p>
                          <p className="text-[10px] text-emerald-200/55">{f.meta}</p>
                        </div>
                      </motion.div>
                    ))}

                    {showScan ? (
                      <div className="space-y-1.5 pt-1">
                        <div className="h-1.5 overflow-hidden rounded-full bg-white/10">
                          <motion.div
                            key={`bar-${tick}`}
                            className="h-full rounded-full bg-gradient-to-r from-[#4ADE80] to-[#F59E0B]"
                            initial={{ width: "0%" }}
                            animate={{ width: phase === "tags" ? "100%" : "72%" }}
                            transition={{ duration: 1.6, ease: "easeInOut" }}
                          />
                        </div>
                        {showTags ? (
                          <div className="flex flex-wrap gap-1">
                            {OCR_TAGS.map((tag, i) => (
                              <motion.span
                                key={tag.label}
                                initial={{ opacity: 0, scale: 0.8 }}
                                animate={{ opacity: 1, scale: 1 }}
                                transition={{ delay: i * 0.12 }}
                                className={`rounded-full px-2 py-0.5 text-[9px] font-black uppercase tracking-wide ring-1 ${tag.color}`}
                              >
                                {tag.label}
                              </motion.span>
                            ))}
                          </div>
                        ) : (
                          <p className="text-center text-[10px] font-bold text-amber-200">
                            Scan OCR Mistral…
                          </p>
                        )}
                      </div>
                    ) : null}
                  </motion.div>
                ) : showCards ? (
                  <motion.div
                    key="cards"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    {CARDS.map((c, i) => (
                      <motion.div
                        key={c.name}
                        layout
                        initial={{ opacity: 0, x: -12 }}
                        animate={{
                          opacity: phase === "fly" && i === 0 ? 0.35 : 1,
                          x: phase === "fly" ? 8 : 0,
                          scale: phase === "fly" ? 0.96 : 1,
                        }}
                        transition={{ delay: i * 0.1 }}
                        className="flex items-center gap-2 rounded-xl bg-emerald-500/15 px-2.5 py-1.5 ring-1 ring-emerald-400/25"
                      >
                        <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/10 text-[10px] font-black text-emerald-100">
                          {c.klass}
                        </span>
                        <div className="min-w-0 flex-1">
                          <p className="truncate text-[11px] font-bold text-white">{c.name}</p>
                          <p className="truncate text-[9px] text-emerald-200/60">{c.file}</p>
                        </div>
                        {phase === "fly" ? (
                          <motion.span
                            animate={{ x: [0, 6, 0] }}
                            transition={{ repeat: Infinity, duration: 0.8 }}
                            className="text-emerald-300"
                          >
                            →
                          </motion.span>
                        ) : null}
                      </motion.div>
                    ))}
                  </motion.div>
                ) : (
                  <motion.p
                    key="empty"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className="flex h-full items-center justify-center text-xs text-emerald-200/50"
                  >
                    Lot traité ✓
                  </motion.p>
                )}
              </AnimatePresence>
            </div>
          </div>

          {/* Colonne droite : dossiers OneDrive */}
          <div className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/5 p-2.5">
            <p className="shrink-0 text-[10px] font-bold uppercase tracking-wider text-emerald-200/70">
              Dossiers OneDrive
            </p>
            <ul className="mt-1.5 flex-1 space-y-1.5">
              {FOLDERS.map((folder, i) => {
                const arriving = phase === "fly" && i === 0;
                const check =
                  phase === "done" ||
                  phase === "folders" ||
                  arriving ||
                  (phase === "fly" && i === 0);
                const lit = showFolders || check;
                return (
                  <motion.li
                    key={folder.name}
                    initial={false}
                    animate={{
                      opacity: lit ? 1 : 0.28,
                      y: check ? 0 : 2,
                    }}
                    transition={{ delay: foldersDone ? i * 0.08 : 0 }}
                    className={`flex items-center gap-2 rounded-xl px-2.5 py-1.5 ${
                      check ? "bg-white/10 ring-1 ring-emerald-400/30" : "bg-white/5"
                    }`}
                  >
                    <span>📁</span>
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-[11px] font-semibold text-white">{folder.name}</p>
                      <p className="truncate text-[9px] text-emerald-200/50">{folder.path}</p>
                    </div>
                    <AnimatePresence>
                      {check ? (
                        <motion.span
                          key="ok"
                          initial={{ scale: 0 }}
                          animate={{ scale: 1 }}
                          exit={{ scale: 0 }}
                          className="text-sm text-[#4ADE80]"
                        >
                          ✓
                        </motion.span>
                      ) : null}
                    </AnimatePresence>
                  </motion.li>
                );
              })}
            </ul>
          </div>
        </div>

        {/* Toast statut */}
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
                  : "bg-white/10 text-emerald-100/85"
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
                  phase === p ? "w-5 bg-[#4ADE80]" : "w-1.5 bg-white/20"
                }`}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
