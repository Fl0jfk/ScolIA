"use client";

import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  DASHBOARD_BONUS_TILES,
  type DashboardPreviewTile,
} from "@/app/lib/dashboard-preview-tiles";
import { MARKETING } from "@/app/lib/marketing-site";
import { TILE_EMOJI } from "@/app/lib/marketing-theme";

function TilePopover({ tile }: { tile: DashboardPreviewTile }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 6, scale: 0.96 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      className="pointer-events-none absolute bottom-full left-1/2 z-20 mb-2 w-[min(18rem,calc(100vw-2rem))] -translate-x-1/2 rounded-xl border-2 border-[#4ADE80]/40 bg-white p-3 text-left shadow-xl shadow-emerald-900/15"
    >
      <p className="text-[10px] font-black uppercase tracking-wider text-[#2F6B4A]">{tile.title}</p>
      <p className="mt-1 text-xs leading-relaxed text-stone-600">{tile.summary}</p>
      <ul className="mt-2 space-y-1">
        {tile.features.slice(0, 3).map((f) => (
          <li key={f} className="flex gap-1.5 text-[11px] leading-snug text-stone-600">
            <span className="shrink-0 text-[#4ADE80]">·</span>
            {f}
          </li>
        ))}
      </ul>
      <div
        className="absolute left-1/2 top-full h-2 w-2 -translate-x-1/2 -translate-y-1 rotate-45 border-b-2 border-r-2 border-[#4ADE80]/40 bg-white"
        aria-hidden
      />
    </motion.div>
  );
}

function TileModal({ tile, onClose }: { tile: DashboardPreviewTile; onClose: () => void }) {
  const emoji = TILE_EMOJI[tile.id];
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-50 flex items-end justify-center bg-[#14231A]/50 p-4 backdrop-blur-sm sm:items-center"
      onClick={onClose}
    >
      <motion.div
        initial={{ opacity: 0, y: 16, scale: 0.98 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 12, scale: 0.98 }}
        role="dialog"
        aria-modal
        className="max-h-[85vh] w-full max-w-md overflow-y-auto rounded-2xl border border-emerald-100 bg-white shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="p-5">
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-start gap-3">
              <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-emerald-100 text-xl">
                {emoji || tile.label.charAt(0)}
              </span>
              <div>
                <p className="text-xs font-bold uppercase tracking-wider text-[#3D8A5C]">
                  {tile.label}
                </p>
                <h3 className="mt-0.5 text-lg font-black text-[#2F6B4A]">{tile.title}</h3>
              </div>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="rounded-lg bg-stone-100 px-2 py-1 text-xs font-bold text-stone-500 hover:bg-stone-200"
              aria-label="Fermer"
            >
              ✕
            </button>
          </div>
          <p className="mt-3 text-sm leading-relaxed text-stone-600">{tile.summary}</p>
          <ul className="mt-4 space-y-2">
            {tile.features.map((f) => (
              <li key={f} className="flex gap-2 text-sm text-stone-700">
                <span className="shrink-0 font-bold text-[#4ADE80]">✓</span>
                {f}
              </li>
            ))}
          </ul>
        </div>
      </motion.div>
    </motion.div>
  );
}

export default function DashboardPreview() {
  const [hoverId, setHoverId] = useState<string | null>(null);
  const [modalTile, setModalTile] = useState<DashboardPreviewTile | null>(null);

  const closeModal = useCallback(() => setModalTile(null), []);

  useEffect(() => {
    if (!modalTile) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeModal();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [modalTile, closeModal]);

  return (
    <>
      <div className="overflow-hidden rounded-2xl border-2 border-emerald-200/80 bg-white shadow-xl shadow-emerald-900/10">
        <div className="flex items-center gap-2 border-b border-emerald-100 bg-gradient-to-r from-emerald-50 to-white px-4 py-3">
          <span className="h-2.5 w-2.5 rounded-full bg-red-400" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400" />
          <span className="h-2.5 w-2.5 rounded-full bg-emerald-500" />
          <span className="ml-3 text-xs font-bold text-stone-600">
            Tableau de bord {MARKETING.productName}
          </span>
        </div>
        <p className="border-b border-emerald-50 bg-emerald-50/50 px-4 py-2 text-center text-[11px] font-semibold text-[#2F6B4A]">
          Survolez ou touchez une tuile — chaque module en détail
        </p>
        <div className="grid grid-cols-2 gap-2.5 p-4 sm:grid-cols-3 sm:gap-3 sm:p-6">
          {DASHBOARD_BONUS_TILES.map((tile) => {
            const isHovered = hoverId === tile.id;
            const emoji = TILE_EMOJI[tile.id];
            return (
              <motion.button
                key={tile.id}
                type="button"
                whileHover={{ scale: 1.03 }}
                whileTap={{ scale: 0.98 }}
                className={`relative overflow-hidden rounded-xl border-2 px-2 py-3 text-center transition ${
                  isHovered
                    ? "border-[#4ADE80] shadow-lg shadow-emerald-900/15"
                    : "border-emerald-100 bg-white hover:border-emerald-300"
                }`}
                onMouseEnter={() => setHoverId(tile.id)}
                onMouseLeave={() => setHoverId(null)}
                onFocus={() => setHoverId(tile.id)}
                onBlur={() => setHoverId(null)}
                onClick={() => setModalTile(tile)}
              >
                {isHovered ? <TilePopover tile={tile} /> : null}
                <div className="mx-auto mb-2 flex h-12 w-12 items-center justify-center rounded-xl bg-emerald-100 text-xl">
                  {emoji || tile.label.charAt(0)}
                </div>
                <p className="text-xs font-black text-[#14231A]">{tile.label}</p>
              </motion.button>
            );
          })}
        </div>
      </div>

      <AnimatePresence>
        {modalTile ? <TileModal tile={modalTile} onClose={closeModal} /> : null}
      </AnimatePresence>
    </>
  );
}
