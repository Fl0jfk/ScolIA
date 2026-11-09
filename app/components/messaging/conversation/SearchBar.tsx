"use client";

import { useEffect, useRef } from "react";
import { IconSearch, IconX } from "../MessagingIcons";
import { IconChevronDown, IconChevronUp } from "./ExtraIcons";

type Props = {
  query: string;
  onQuery: (q: string) => void;
  /** Position (1-based) du résultat courant, 0 si aucun. */
  position: number;
  total: number;
  onPrev: () => void;
  onNext: () => void;
  onClose: () => void;
};

/** Recherche dans les messages chargés de la conversation. */
export default function SearchBar({ query, onQuery, position, total, onPrev, onNext, onClose }: Props) {
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const btn =
    "rounded-full p-1.5 text-[var(--dash-ink)] outline-none transition hover:bg-black/[0.06] focus-visible:ring-2 focus-visible:ring-[var(--dash-ink)]/50 disabled:opacity-40";

  return (
    <div
      role="search"
      className="flex items-center gap-1.5 border-b border-black/5 bg-white/70 px-3 py-1.5"
    >
      <IconSearch className="h-4 w-4 shrink-0 text-[var(--dash-mid)]" />
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => onQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            onClose();
          } else if (e.key === "Enter") {
            e.preventDefault();
            if (e.shiftKey) onPrev();
            else onNext();
          }
        }}
        aria-label="Rechercher dans la conversation"
        placeholder="Rechercher dans la conversation"
        className="min-w-0 flex-1 bg-transparent py-1 text-sm text-[var(--dash-ink)] outline-none placeholder:text-neutral-500"
      />
      <span className="shrink-0 text-[11px] tabular-nums text-[var(--dash-mid)]" aria-live="polite">
        {query.trim() ? (total > 0 ? `${position}/${total}` : "Aucun résultat") : ""}
      </span>
      <button type="button" className={btn} aria-label="Résultat précédent" disabled={total === 0} onClick={onPrev}>
        <IconChevronUp className="h-4 w-4" />
      </button>
      <button type="button" className={btn} aria-label="Résultat suivant" disabled={total === 0} onClick={onNext}>
        <IconChevronDown className="h-4 w-4" />
      </button>
      <button type="button" className={btn} aria-label="Fermer la recherche" onClick={onClose}>
        <IconX className="h-4 w-4" />
      </button>
    </div>
  );
}
