"use client";

import { useState } from "react";
import { IconX } from "../MessagingIcons";
import { IconChevronDown, IconChevronUp } from "./ExtraIcons";

/** Carte repliable « Résumé ScolIA » (généré par IA, à vérifier). */
export default function SummaryCard({ text, onClose }: { text: string; onClose: () => void }) {
  const [open, setOpen] = useState(true);
  return (
    <section
      aria-label="Résumé ScolIA"
      className="border-b border-black/5 bg-[var(--dash-lime)]/15 px-3.5 py-2"
    >
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => setOpen((v) => !v)}
          aria-expanded={open}
          className="flex min-w-0 flex-1 items-center gap-1.5 rounded text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50"
        >
          <span className="text-xs font-semibold text-[var(--dash-ink)]">Résumé ScolIA</span>
          <span className="truncate text-[11px] text-[var(--dash-mid)]">généré par IA, à vérifier</span>
          {open ? (
            <IconChevronUp className="ml-auto h-3.5 w-3.5 shrink-0 text-[var(--dash-mid)]" />
          ) : (
            <IconChevronDown className="ml-auto h-3.5 w-3.5 shrink-0 text-[var(--dash-mid)]" />
          )}
        </button>
        <button
          type="button"
          onClick={onClose}
          aria-label="Fermer le résumé"
          className="rounded-full p-1 text-[var(--dash-mid)] outline-none transition hover:bg-black/[0.06] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50"
        >
          <IconX className="h-3.5 w-3.5" />
        </button>
      </div>
      {open ? (
        <p className="mt-1.5 max-h-40 overflow-y-auto whitespace-pre-wrap break-words text-[13px] leading-relaxed text-[var(--dash-ink)]">
          {text}
        </p>
      ) : null}
    </section>
  );
}
