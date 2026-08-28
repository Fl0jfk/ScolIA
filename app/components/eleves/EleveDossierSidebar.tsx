"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "next/link";

type Classmate = { id: string; nom: string; prenom: string };

type SearchRow = Classmate & { classe?: string | null };

type Props = {
  currentEleveId: string;
  classe: string | null;
  classmates: Classmate[];
  dossierHref: (eleveId: string) => string;
};

export default function EleveDossierSidebar({
  currentEleveId,
  classe,
  classmates,
  dossierHref,
}: Props) {
  const [q, setQ] = useState("");
  const [searchPool, setSearchPool] = useState<SearchRow[] | null>(null);
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
    const needle = q.trim();
    if (needle.length >= 2) void loadSearchPool();
  }, [q, loadSearchPool]);

  const searchResults = useMemo(() => {
    const needle = q.trim().toLowerCase();
    if (needle.length < 2) return [];
    const pool = searchPool ?? [];
    return pool
      .filter((e) => {
        if (e.id === currentEleveId) return false;
        return `${e.prenom} ${e.nom} ${e.classe || ""}`.toLowerCase().includes(needle);
      })
      .slice(0, 8);
  }, [q, searchPool, currentEleveId]);

  const sortedClassmates = useMemo(
    () =>
      [...classmates].sort((a, b) =>
        `${a.nom} ${a.prenom}`.localeCompare(`${b.nom} ${b.prenom}`, "fr", {
          sensitivity: "base",
        }),
      ),
    [classmates],
  );

  return (
    <aside className="space-y-4">
      <div className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm">
        <label htmlFor="eleve-sidebar-search" className="mb-2 block text-xs font-bold text-slate-700">
          Rechercher un autre élève
        </label>
        <input
          id="eleve-sidebar-search"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Nom, prénom…"
          autoComplete="off"
          className="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm outline-none focus:border-indigo-300 focus:ring-2 focus:ring-indigo-100"
        />
        {q.trim().length >= 2 ? (
          <ul className="mt-2 max-h-48 overflow-y-auto rounded-xl border border-slate-100 bg-slate-50/80">
            {searchBusy && !searchPool ? (
              <li className="px-3 py-2 text-xs text-slate-500">Recherche…</li>
            ) : null}
            {searchResults.map((e) => (
              <li key={e.id}>
                <Link
                  href={dossierHref(e.id)}
                  className="block px-3 py-2 text-sm font-medium text-slate-800 hover:bg-indigo-50 hover:text-indigo-900"
                >
                  {e.prenom} {e.nom}
                  {e.classe ? (
                    <span className="ml-1 text-xs font-normal text-slate-500">· {e.classe}</span>
                  ) : null}
                </Link>
              </li>
            ))}
            {!searchBusy && searchPool && searchResults.length === 0 ? (
              <li className="px-3 py-2 text-xs text-slate-500">Aucun élève trouvé.</li>
            ) : null}
          </ul>
        ) : null}
      </div>

      {classe ? (
        <div className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm">
          <h2 className="text-xs font-bold uppercase tracking-wide text-slate-500">
            Classe {classe}
          </h2>
          <p className="mt-0.5 text-[11px] text-slate-400">
            {sortedClassmates.length} autre{sortedClassmates.length > 1 ? "s" : ""} élève
            {sortedClassmates.length > 1 ? "s" : ""}
          </p>
          <ul className="mt-3 max-h-[min(60vh,28rem)] overflow-y-auto space-y-0.5">
            {sortedClassmates.map((c) => (
              <li key={c.id}>
                <Link
                  href={dossierHref(c.id)}
                  className="block rounded-lg px-2 py-1.5 text-sm text-slate-700 transition hover:bg-slate-100 hover:text-slate-900"
                >
                  <span className="font-semibold">
                    {c.nom.toUpperCase()}
                  </span>{" "}
                  {c.prenom}
                </Link>
              </li>
            ))}
            {sortedClassmates.length === 0 ? (
              <li className="text-xs text-slate-500 py-2">Aucun autre élève dans cette classe.</li>
            ) : null}
          </ul>
        </div>
      ) : null}
    </aside>
  );
}
