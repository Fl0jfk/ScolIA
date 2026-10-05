"use client";

import { useCallback, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";

type Ligne = {
  eleveId: string;
  nom: string;
  prenom: string;
  classe: string | null;
  chambreLabel: string;
  batimentLabel: string;
  droitSoir: boolean;
  prisSoir: boolean;
  statut: "attendu_pris" | "attendu_manquant" | "imprevu" | "en_sortie";
  sourceDroit: "grille" | "regime" | null;
};

type Payload = {
  date: string;
  jourCle: string | null;
  attendus: number;
  pris: number;
  manquants: number;
  enSortie: number;
  lignes: Ligne[];
};

function todayParis(): string {
  return new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

function normalizeIsoDate(raw: string | null): string | null {
  if (!raw) return null;
  const m = /^(\d{4}-\d{2}-\d{2})$/.exec(raw.trim());
  return m ? m[1]! : null;
}

const STATUT_LABEL: Record<Ligne["statut"], string> = {
  attendu_pris: "Attendu · pris",
  attendu_manquant: "Attendu · manquant",
  imprevu: "Imprévu",
  en_sortie: "En sortie",
};

function statutClass(s: Ligne["statut"]): string {
  if (s === "attendu_pris") return "border-emerald-200 bg-emerald-50 text-emerald-950";
  if (s === "attendu_manquant") return "border-amber-200 bg-amber-50 text-amber-950";
  if (s === "en_sortie") return "border-slate-200 bg-slate-50 text-slate-700";
  return "border-rose-200 bg-rose-50 text-rose-950";
}

export default function InternatRepasSoirClient() {
  const searchParams = useSearchParams();
  const dateFromUrl = normalizeIsoDate(searchParams.get("date"));
  const [date, setDate] = useState(dateFromUrl ?? todayParis);
  const [data, setData] = useState<Payload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (dateFromUrl && dateFromUrl !== date) setDate(dateFromUrl);
  }, [dateFromUrl, date]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    const res = await fetch(`/api/internat/repas-soir?date=${encodeURIComponent(date)}`, {
      credentials: "include",
    });
    const json = (await res.json().catch(() => ({}))) as Payload & { error?: string };
    if (!res.ok) {
      setError(json.error || "Chargement impossible.");
      setData(null);
      setLoading(false);
      return;
    }
    setData(json);
    setLoading(false);
  }, [date]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="space-y-6">
      <p className="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700">
        Internes logés (lit daté) : droit soir (grille / régime) vs passage self ≥ 16 h. Les élèves
        en sortie week-end apparaissent à part.
      </p>

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <label className="text-sm font-semibold text-slate-800">
          Date
          <input
            type="date"
            value={date}
            onChange={(e) => {
              const v = normalizeIsoDate(e.target.value);
              if (v) setDate(v);
            }}
            lang="fr-CA"
            className="mt-1 block rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
          />
        </label>
        <button
          type="button"
          onClick={() => void load()}
          className="mt-3 rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-800"
        >
          Actualiser
        </button>
      </section>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-900">
          {error}
        </p>
      ) : null}

      {data ? (
        <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
          <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
            Synthèse{data.jourCle ? ` — ${data.jourCle}` : " — week-end"}
          </h2>
          <dl className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <div className="rounded-2xl border border-slate-100 bg-slate-50 px-3 py-3">
              <dt className="text-xs text-slate-500">Attendus</dt>
              <dd className="mt-1 text-2xl font-bold">{data.attendus}</dd>
            </div>
            <div className="rounded-2xl border border-emerald-100 bg-emerald-50/60 px-3 py-3">
              <dt className="text-xs text-emerald-800">Pris</dt>
              <dd className="mt-1 text-2xl font-bold text-emerald-950">{data.pris}</dd>
            </div>
            <div className="rounded-2xl border border-amber-100 bg-amber-50/60 px-3 py-3">
              <dt className="text-xs text-amber-800">Manquants</dt>
              <dd className="mt-1 text-2xl font-bold text-amber-950">{data.manquants}</dd>
            </div>
            <div className="rounded-2xl border border-slate-100 bg-slate-50 px-3 py-3">
              <dt className="text-xs text-slate-500">En sortie</dt>
              <dd className="mt-1 text-2xl font-bold">{data.enSortie}</dd>
            </div>
          </dl>
        </section>
      ) : null}

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Détail</h2>
        {loading ? (
          <p className="mt-3 text-sm text-slate-500">Chargement…</p>
        ) : !data || data.lignes.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucune ligne pour ce jour.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {data.lignes.map((l) => (
              <li
                key={l.eleveId}
                className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm"
              >
                <div>
                  <p className="font-semibold text-slate-900">
                    {l.prenom} {l.nom}
                    {l.classe ? (
                      <span className="ml-2 text-xs font-medium text-slate-500">{l.classe}</span>
                    ) : null}
                  </p>
                  <p className="text-xs text-slate-500">
                    {l.batimentLabel} / {l.chambreLabel}
                    {l.sourceDroit ? ` · droit ${l.sourceDroit}` : ""}
                  </p>
                </div>
                <span
                  className={`rounded-full border px-2.5 py-1 text-xs font-bold ${statutClass(l.statut)}`}
                >
                  {STATUT_LABEL[l.statut]}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
