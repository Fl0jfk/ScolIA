"use client";

import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  WORKFLOW_ANIMATION_BODY,
  WORKFLOW_ANIMATION_INNER,
  WORKFLOW_ANIMATION_SHELL,
} from "@/app/lib/marketing-theme";

const PHASES = ["drop", "scan", "sort", "done"] as const;
type Phase = (typeof PHASES)[number];
const PHASE_MS = 2400;

const BATCH = [
  { name: "bulletins_T2_6A.pdf", meta: "Lot · 28 pages" },
  { name: "releve_notes_BAC.pdf", meta: "Relevé bac" },
];

const FOLDERS = [
  { name: "Dupont Marie — 6A", file: "bulletin.pdf" },
  { name: "Martin Lucas — 5B", file: "bulletin.pdf" },
  { name: "Bernard Léa — Tle", file: "releve_bac.pdf" },
];

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

  return (
    <div
      className={`${WORKFLOW_ANIMATION_SHELL} border border-emerald-400/30 bg-gradient-to-br from-[#1E4A32] via-[#2F6B4A] to-[#1A3D2B] shadow-emerald-900/30`}
    >
      <div className="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-[#4ADE80]/20 blur-2xl" />
      <div className={`${WORKFLOW_ANIMATION_INNER} bg-[#0F2318]/40`}>
        <div className="mb-3 flex shrink-0 items-center gap-2 border-b border-white/10 pb-3">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/80" />
          <span className="h-2.5 w-2.5 rounded-full bg-emerald-400/80" />
          <span className="ml-2 text-xs font-semibold text-emerald-100/80">Documents élèves · IA</span>
          <motion.span
            key={tick}
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            className="ml-auto rounded-full bg-[#F59E0B]/20 px-2 py-0.5 text-[10px] font-black uppercase tracking-wider text-amber-200"
          >
            Mistral OCR
          </motion.span>
        </div>

        <div className={`${WORKFLOW_ANIMATION_BODY} grid gap-3 sm:grid-cols-[1fr_auto_1fr]`}>
          <div className="relative flex min-h-[11rem] flex-col rounded-2xl border-2 border-dashed border-emerald-400/40 bg-white/5 p-3">
            <p className="text-[10px] font-bold uppercase tracking-wider text-emerald-200/70">
              Dépôt — lot
            </p>
            <div className="mt-2 flex-1 space-y-1.5">
              <AnimatePresence mode="wait">
                {phase === "drop" || phase === "scan" ? (
                  <motion.div
                    key="files"
                    initial={{ opacity: 0, y: -6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                    className="space-y-1.5"
                  >
                    {BATCH.map((f) => (
                      <div
                        key={f.name}
                        className="flex items-center gap-2 rounded-xl bg-white/10 px-2.5 py-2"
                      >
                        <div className="flex h-8 w-7 items-center justify-center rounded-md bg-red-500/90 text-[9px] font-black text-white">
                          PDF
                        </div>
                        <div className="min-w-0">
                          <p className="truncate text-xs font-bold text-white">{f.name}</p>
                          <p className="text-[10px] text-emerald-200/55">{f.meta}</p>
                        </div>
                      </div>
                    ))}
                  </motion.div>
                ) : (
                  <motion.p
                    key="empty"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className="py-6 text-center text-xs text-emerald-200/50"
                  >
                    Lot traité ✓
                  </motion.p>
                )}
              </AnimatePresence>
            </div>
            {phase === "scan" ? (
              <div className="mt-2 space-y-1">
                <div className="h-1.5 overflow-hidden rounded-full bg-white/10">
                  <motion.div
                    className="h-full rounded-full bg-gradient-to-r from-[#4ADE80] to-[#F59E0B]"
                    initial={{ width: "0%" }}
                    animate={{ width: "100%" }}
                    transition={{ duration: 1.9, ease: "easeInOut" }}
                  />
                </div>
                <p className="text-center text-[10px] font-bold text-amber-200">
                  OCR · matching élèves…
                </p>
              </div>
            ) : null}
          </div>

          <div className="hidden items-center sm:flex">
            <motion.span
              animate={{ opacity: phase === "drop" ? 0.3 : 1, x: [0, 3, 0] }}
              transition={{ repeat: Infinity, duration: 1.1 }}
              className="text-2xl text-[#4ADE80]"
            >
              →
            </motion.span>
          </div>

          <div className="flex min-h-[11rem] flex-col rounded-2xl border border-white/10 bg-white/5 p-3">
            <p className="text-[10px] font-bold uppercase tracking-wider text-emerald-200/70">
              Dossiers OneDrive
            </p>
            <ul className="mt-2 flex-1 space-y-1.5">
              {FOLDERS.map((folder, i) => {
                const lit = phase === "sort" || phase === "done";
                const check = phase === "done" || (phase === "sort" && i < 2);
                return (
                  <li
                    key={folder.name}
                    className={`flex items-center gap-2 rounded-xl px-2.5 py-1.5 transition-all ${
                      lit ? "bg-white/10 opacity-100" : "bg-white/5 opacity-30"
                    }`}
                  >
                    <span>📁</span>
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-xs font-semibold text-white">{folder.name}</p>
                      {check ? (
                        <p className="truncate text-[10px] text-emerald-200/70">{folder.file}</p>
                      ) : null}
                    </div>
                    {check ? <span className="text-[#4ADE80]">✓</span> : null}
                  </li>
                );
              })}
            </ul>
            <p
              className={`mt-1 text-center text-xs font-black transition-opacity ${
                phase === "done" ? "text-[#4ADE80] opacity-100" : "opacity-0"
              }`}
            >
              Rangé automatiquement
            </p>
          </div>
        </div>

        <div className="mt-3 flex justify-center gap-1.5">
          {PHASES.map((p) => (
            <span
              key={p}
              className={`h-1.5 rounded-full transition-all ${
                phase === p ? "w-6 bg-[#4ADE80]" : "w-1.5 bg-white/20"
              }`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
