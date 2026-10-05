"use client";

import { useCallback, useEffect, useMemo, useState } from "react";

export type EleveSearchHit = {
  id: string;
  nom: string;
  prenom: string;
  classe?: string | null;
};

type Props = {
  onSelect: (eleveId: string) => void;
  excludeId?: string | null;
  compact?: boolean;
  inputId?: string;
  placeholder?: string;
};

export default function GlobalEleveSearch({
  onSelect,
  excludeId = null,
  compact = false,
  inputId = "global-eleve-search",
  placeholder = "Rechercher un élève…",
}: Props) {
  const [q, setQ] = useState("");
  const [searchPool, setSearchPool] = useState<EleveSearchHit[] | null>(null);
  const [searchBusy, setSearchBusy] = useState(false);

  const loadSearchPool = useCallback(async () => {
    if (searchPool !== null) return;
    setSearchBusy(true);
    try {
      const res = await fetch("/api/eleves/dossiers/list?status=inscrit", { cache: "no-store" });
      if (!res.ok) return;
      const j = (await res.json()) as {
        eleves?: Array<{ id: string; nom: string; prenom: string; classe?: string | null }>;
      };
      setSearchPool(
        (j.eleves || []).map((e) => ({
          id: e.id,
          nom: e.nom,
          prenom: e.prenom,
          classe: e.classe,
        })),
      );
    } catch {
      /* recherche secondaire */
    } finally {
      setSearchBusy(false);
    }
  }, [searchPool]);

  useEffect(() => {
    if (q.trim().length >= 2) void loadSearchPool();
  }, [q, loadSearchPool]);

  const searchResults = useMemo(() => {
    const needle = q.trim().toLowerCase();
    if (needle.length < 2) return [];
    const pool = searchPool ?? [];
    return pool
      .filter((e) => {
        if (excludeId && e.id === excludeId) return false;
        return `${e.prenom} ${e.nom} ${e.classe || ""}`.toLowerCase().includes(needle);
      })
      .slice(0, 12);
  }, [q, searchPool, excludeId]);

  function select(id: string) {
    onSelect(id);
    setQ("");
  }

  return (
    <div className={compact ? "" : "space-y-1.5"}>
      {!compact ? (
        <label htmlFor={inputId} className="block text-[10px] font-semibold uppercase tracking-[0.16em] text-[var(--dash-mid)]">
          Élèves
        </label>
      ) : null}
      <div className="relative">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          strokeWidth={2}
          stroke="currentColor"
          className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-neutral-400"
          aria-hidden
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z"
          />
        </svg>
        <input
          id={inputId}
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder={placeholder}
          autoComplete="off"
          className="w-full rounded-xl border border-black/8 bg-white py-2 pl-9 pr-3 text-sm font-medium text-[var(--dash-ink)] outline-none transition placeholder:text-neutral-400 focus:border-[var(--dash-ink)] focus:ring-2 focus:ring-[color:var(--dash-lime)]/60"
        />
      </div>
      {q.trim().length >= 2 ? (
        <ul className="mt-1.5 max-h-52 overflow-y-auto rounded-xl border border-black/6 bg-white shadow-sm">
          {searchBusy && !searchPool ? (
            <li className="px-3 py-2 text-xs text-neutral-500">Recherche…</li>
          ) : null}
          {searchResults.map((e) => (
            <li key={e.id}>
              <button
                type="button"
                onClick={() => select(e.id)}
                className="flex w-full items-center justify-between gap-2 px-3 py-2 text-left text-sm font-medium text-[var(--dash-ink)] transition hover:bg-[color:var(--dash-soft-muted)]"
              >
                <span>
                  {e.prenom} {e.nom}
                </span>
                {e.classe ? (
                  <span className="shrink-0 rounded-full bg-[color:var(--dash-lime)]/80 px-2 py-0.5 text-[10px] font-bold text-[var(--dash-ink)]">
                    {e.classe}
                  </span>
                ) : null}
              </button>
            </li>
          ))}
          {!searchBusy && searchPool && searchResults.length === 0 ? (
            <li className="px-3 py-2 text-xs text-neutral-500">Aucun élève trouvé.</li>
          ) : null}
        </ul>
      ) : null}
    </div>
  );
}
