"use client";

import { useMemo, useState } from "react";
import {
  communeRecapToCsv,
  type CommuneRecap,
} from "@/app/lib/travels-commune-recap";

/** Récapitulatif des élèves partis en voyage, par commune du foyer (année scolaire en cours). */
export default function TravelsCommuneRecap() {
  const [recap, setRecap] = useState<CommuneRecap | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [openCommune, setOpenCommune] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/travels/recap-communes", { cache: "no-store" });
      const json = (await res.json().catch(() => ({}))) as CommuneRecap & { error?: string };
      if (!res.ok) throw new Error(json.error || "Impossible de charger le récapitulatif.");
      setRecap(json);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Impossible de charger le récapitulatif.");
    } finally {
      setLoading(false);
    }
  };

  const communes = useMemo(() => {
    if (!recap) return [];
    const q = query.trim().toLowerCase();
    if (!q) return recap.communes;
    return recap.communes.filter((g) => g.commune.toLowerCase().includes(q));
  }, [recap, query]);

  const downloadCsv = () => {
    if (!recap) return;
    const blob = new Blob([`﻿${communeRecapToCsv(recap)}`], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `voyages-par-commune-${recap.schoolYearLabel}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <section className="mt-10 rounded-2xl border border-slate-200 bg-white p-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-bold text-slate-900">Récapitulatif par commune</h2>
          <p className="text-xs text-slate-500">
            Élèves partis en sortie ou voyage depuis le début de l&apos;année scolaire, regroupés par commune du foyer.
          </p>
        </div>
        <div className="flex gap-2">
          {recap && (
            <button
              type="button"
              onClick={downloadCsv}
              className="rounded-xl border border-slate-200 px-3 py-2 text-sm font-bold text-slate-700 hover:border-slate-400"
            >
              Exporter (Excel)
            </button>
          )}
          <button
            type="button"
            onClick={() => void load()}
            disabled={loading}
            className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-60"
          >
            {loading ? "Chargement…" : recap ? "Actualiser" : "Afficher le récapitulatif"}
          </button>
        </div>
      </div>

      {error && <p className="mt-3 text-sm text-rose-600">{error}</p>}

      {recap && (
        <div className="mt-4">
          <p className="text-sm text-slate-600">
            Année {recap.schoolYearLabel} · {recap.tripCount} sortie(s) avec liste d&apos;élèves ·{" "}
            {recap.nbEleves} élève(s) · {recap.nbParticipations} participation(s)
          </p>
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Filtrer par commune…"
            className="mt-3 w-full max-w-sm rounded-xl border border-slate-200 px-3 py-2 text-sm outline-none focus:border-indigo-400"
          />
          {communes.length === 0 ? (
            <p className="mt-4 text-sm text-slate-500">Aucune commune à afficher.</p>
          ) : (
            <ul className="mt-4 divide-y divide-slate-100 rounded-xl border border-slate-100">
              {communes.map((g) => {
                const open = openCommune === g.commune;
                return (
                  <li key={g.commune}>
                    <button
                      type="button"
                      onClick={() => setOpenCommune(open ? null : g.commune)}
                      aria-expanded={open}
                      className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left hover:bg-slate-50"
                    >
                      <span className="text-sm font-bold text-slate-800">{g.commune}</span>
                      <span className="text-xs text-slate-500">
                        {g.nbEleves} élève(s) · {g.nbParticipations} participation(s) {open ? "▲" : "▼"}
                      </span>
                    </button>
                    {open && (
                      <div className="overflow-x-auto px-4 pb-4">
                        <table className="w-full text-left text-sm">
                          <thead className="text-xs uppercase text-slate-400">
                            <tr>
                              <th className="py-1 pr-3">Élève</th>
                              <th className="py-1 pr-3">Classe</th>
                              <th className="py-1">Voyages</th>
                            </tr>
                          </thead>
                          <tbody>
                            {g.eleves.map((e) => (
                              <tr key={e.key} className="border-t border-slate-50 align-top">
                                <td className="py-1.5 pr-3 font-medium text-slate-800">
                                  {e.nom} {e.prenom}
                                </td>
                                <td className="py-1.5 pr-3 text-slate-600">{e.classe || "—"}</td>
                                <td className="py-1.5 text-slate-600">
                                  {e.trips.map((t) => `${t.title} (${t.dateLabel})`).join(" · ")}
                                </td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
