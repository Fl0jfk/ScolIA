"use client";

import { motion } from "framer-motion";

const TILES = [
  {
    id: "docs",
    emoji: "📄",
    label: "Documents IA",
    hint: "OCR · classement",
    gradient: "from-[#1E4A32] to-[#2F6B4A]",
    ring: "ring-emerald-400/30",
  },
  {
    id: "travels",
    emoji: "🚌",
    label: "Sorties",
    hint: "Devis · compta",
    gradient: "from-[#1E3A5F] to-[#234B73]",
    ring: "ring-sky-400/30",
  },
  {
    id: "rooms",
    emoji: "🚪",
    label: "Salles",
    hint: "Temps réel",
    gradient: "from-[#3B2F5F] to-[#4C3D7A]",
    ring: "ring-violet-400/30",
  },
  {
    id: "rh",
    emoji: "👥",
    label: "RH",
    hint: "Absences · arrivées",
    gradient: "from-[#5C2D3A] to-[#6B3A4A]",
    ring: "ring-rose-400/30",
  },
] as const;

/** Mosaïque hero — aperçu des 4 piliers sans rejouer le workflow Docs. */
export default function HeroPillarMosaic() {
  return (
    <div className="relative mx-auto grid max-w-2xl grid-cols-2 gap-3 sm:gap-4">
      <div className="pointer-events-none absolute -inset-6 rounded-[2rem] bg-gradient-to-br from-emerald-200/30 via-transparent to-sky-200/20 blur-2xl" />
      {TILES.map((t, i) => (
        <motion.div
          key={t.id}
          initial={{ opacity: 0, y: 16, scale: 0.96 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ delay: 0.12 + i * 0.08, duration: 0.55, ease: [0.22, 1, 0.36, 1] }}
          whileHover={{ y: -3 }}
          className={`relative overflow-hidden rounded-2xl bg-gradient-to-br ${t.gradient} p-4 text-left text-white shadow-lg ring-1 ${t.ring} sm:p-5`}
        >
          <motion.span
            animate={{ y: [0, -4, 0] }}
            transition={{ repeat: Infinity, duration: 3.2 + i * 0.4, ease: "easeInOut" }}
            className="text-2xl"
          >
            {t.emoji}
          </motion.span>
          <p className="mt-2 text-sm font-black tracking-tight">{t.label}</p>
          <p className="mt-0.5 text-[11px] text-white/70">{t.hint}</p>
          <motion.div
            className="pointer-events-none absolute -right-4 -top-4 h-16 w-16 rounded-full bg-white/10 blur-xl"
            animate={{ opacity: [0.3, 0.55, 0.3] }}
            transition={{ repeat: Infinity, duration: 2.8 + i * 0.3 }}
          />
        </motion.div>
      ))}
    </div>
  );
}
