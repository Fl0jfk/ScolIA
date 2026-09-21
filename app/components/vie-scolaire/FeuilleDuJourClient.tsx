"use client";

import { useCallback, useEffect, useState } from "react";
import { parisDateKey } from "@/app/lib/paris-time";
import type { FeuilleDuJourRow, FeuilleDuJourSummary } from "@/app/lib/feuille-du-jour";

function todayIso(): string {
  return parisDateKey(new Date());
}

function tagClass(tag: string): string {
  switch (tag) {
    case "en_sortie":
      return "bg-sky-50 text-sky-900 border-sky-200";
    case "en_stage":
      return "bg-violet-50 text-violet-900 border-violet-200";
    case "absent_vs":
      return "bg-amber-50 text-amber-950 border-amber-200";
    case "internat":
      return "bg-indigo-50 text-indigo-900 border-indigo-200";
    case "en_cours":
      return "bg-emerald-50 text-emerald-900 border-emerald-200";
    default:
      return "bg-slate-50 text-slate-700 border-slate-200";
  }
}

export default function FeuilleDuJourClient() {
  const [date, setDate] = useState(todayIso);
  const [q, setQ] = useState("");
  const [classe, setClasse] = useState("");
  const [rows, setRows] = useState<FeuilleDuJourRow[]>([]);
  const [summary, setSummary] = useState<FeuilleDuJourSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loadedOnce, setLoadedOnce] = useState(false);

  const load = useCallback(async () => {
    const params = new URLSearchParams({ date });
    if (classe.trim()) params.set("classe", classe.trim());
    else if (q.trim().length >= 2) params.set("q", q.trim());
    else {
      setError("Indiquez une classe ou un nom (2 lettres min.).");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(`/api/vie-scolaire/presence-jour?${params}`, {
        cache: "no-store",
      });
      const data = (await res.json()) as {
        error?: string;
        rows?: FeuilleDuJourRow[];
        summary?: FeuilleDuJourSummary;
      };
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setRows(data.rows || []);
      setSummary(data.summary || null);
      setLoadedOnce(true);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
      setRows([]);
      setSummary(null);
    } finally {
      setBusy(false);
    }
  }, [date, q, classe]);

  useEffect(() => {
    if (classe.trim() || q.trim().length >= 2) {
      const t = setTimeout(() => {
        void load();
      }, 200);
      return () => clearTimeout(t);
    }
    return undefined;
  }, [classe, q, date, load]);

  return (
    <div className="space-y-4">
      <div className="rounded-2xl border border-sky-100 bg-sky-50/70 px-4 py-3 text-sm text-sky-950">
        <p className="font-semibold">Où est l’élève aujourd’hui ?</p>
        <p className="mt-1 opacity-90">
          Lecture live du cerveau (occupancy) : sortie scolaire, stage, absence VS… Une sortie{" "}
          <strong>n’est pas</strong> une absence bulletin.
        </p>
      </div>

      <section className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
        <div className="grid gap-3 sm:grid-cols-3">
          <label className="block text-sm font-semibold text-slate-800">
            Date
            <input
              type="date"
              value={date}
              onChange={(e) => setDate(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 font-normal"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Classe
            <input
              value={classe}
              onChange={(e) => {
                setClasse(e.target.value);
                if (e.target.value.trim()) setQ("");
              }}
              placeholder="ex. 4B"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 font-normal"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Chercher un élève
            <input
              value={q}
              onChange={(e) => {
                setQ(e.target.value);
                if (e.target.value.trim()) setClasse("");
              }}
              placeholder="Nom ou prénom…"
              autoComplete="off"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 font-normal"
            />
          </label>
        </div>
        <button
          type="button"
          disabled={busy}
          onClick={() => void load()}
          className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-semibold text-white hover:bg-slate-800 disabled:opacity-50"
        >
          {busy ? "Lecture…" : "Actualiser"}
        </button>
      </section>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-900">
          {error}
        </p>
      ) : null}

      {summary && loadedOnce ? (
        <div className="flex flex-wrap gap-2 text-xs font-semibold">
          <span className="rounded-full border border-slate-200 bg-white px-3 py-1 text-slate-700">
            {summary.total} élève(s)
          </span>
          {summary.enSortie > 0 ? (
            <span className="rounded-full border border-sky-200 bg-sky-50 px-3 py-1 text-sky-900">
              {summary.enSortie} en sortie
            </span>
          ) : null}
          {summary.absentVs > 0 ? (
            <span className="rounded-full border border-amber-200 bg-amber-50 px-3 py-1 text-amber-950">
              {summary.absentVs} absent(s) VS
            </span>
          ) : null}
          {summary.enStage > 0 ? (
            <span className="rounded-full border border-violet-200 bg-violet-50 px-3 py-1 text-violet-900">
              {summary.enStage} en stage
            </span>
          ) : null}
        </div>
      ) : null}

      {loadedOnce && rows.length === 0 && !error ? (
        <p className="text-sm text-slate-600">Aucun élève pour cette recherche.</p>
      ) : null}

      {rows.length > 0 ? (
        <ul className="divide-y divide-slate-100 overflow-hidden rounded-2xl border border-slate-200 bg-white">
          {rows.map((r) => (
            <li key={r.eleveId} className="flex flex-col gap-1 px-4 py-3 sm:flex-row sm:items-start sm:justify-between">
              <div>
                <p className="font-semibold text-slate-900">{r.displayName}</p>
                <p className="text-sm text-slate-500">
                  {r.classe || "Classe —"}
                  {r.source ? ` · ${r.source}` : ""}
                </p>
                <p className="mt-1 text-sm text-slate-700">{r.explanationFr}</p>
                {r.detailFr && r.detailFr !== r.explanationFr ? (
                  <p className="text-xs text-slate-500">{r.detailFr}</p>
                ) : null}
              </div>
              <div className="shrink-0 space-y-1 sm:text-right">
                <span
                  className={`inline-flex rounded-md border px-2 py-0.5 text-xs font-bold ${tagClass(r.tag)}`}
                >
                  {r.labelFr}
                </span>
                {r.excludeFromBulletin ? (
                  <p className="text-[11px] font-semibold text-sky-800">Hors bulletin</p>
                ) : null}
              </div>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
