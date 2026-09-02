"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { parisDateKey } from "@/app/lib/paris-time";
import type { AccueilBoardKind, AccueilBoardRow } from "@/app/lib/accueil-absences-types";

type KindFilter = "tous" | AccueilBoardKind;

function kindLabel(kind: AccueilBoardKind, eleveNature?: string | null): string {
  if (kind === "eleve") return eleveNature === "retard" ? "Retard" : "Élève";
  if (kind === "professeur") return "Professeur";
  return "Personnel OGEC";
}

function kindBadgeClass(kind: AccueilBoardKind, eleveNature?: string | null): string {
  if (kind === "eleve" && eleveNature === "retard") return "bg-violet-100 text-violet-800";
  if (kind === "eleve") return "bg-sky-100 text-sky-800";
  if (kind === "professeur") return "bg-violet-100 text-violet-800";
  return "bg-amber-100 text-amber-800";
}

function formatPeriod(row: AccueilBoardRow): string {
  const start = row.dateDebut;
  const end = row.dateFin;
  if (row.heureDebut && row.heureFin && start === end) {
    return `${start} · ${row.heureDebut}–${row.heureFin}`;
  }
  if (start === end) return start;
  return `${start} → ${end}`;
}

export default function AccueilAbsencesConsultationClient() {
  const [date, setDate] = useState(() => parisDateKey(new Date()));
  const [kindFilter, setKindFilter] = useState<KindFilter>("eleve");
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<AccueilBoardRow[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch(
        `/api/vie-scolaire/absences-accueil?date=${encodeURIComponent(date)}`,
        { cache: "no-store" },
      );
      const data = (await res.json()) as { error?: string; rows?: AccueilBoardRow[] };
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setRows(data.rows || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, [date]);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = useMemo(() => {
    const needle = query
      .trim()
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "");
    let list = rows;
    if (kindFilter !== "tous") {
      list = list.filter((r) => r.kind === kindFilter);
    }
    if (!needle) return list;
    return list.filter((r) => {
      const hay = `${r.displayName} ${r.subtitle} ${r.motif || ""}`
        .toLowerCase()
        .normalize("NFD")
        .replace(/[\u0300-\u036f]/g, "");
      return hay.includes(needle);
    });
  }, [rows, kindFilter, query]);

  const counts = useMemo(() => {
    const eleves = rows.filter((r) => r.kind === "eleve").length;
    const profs = rows.filter((r) => r.kind === "professeur").length;
    return { total: rows.length, eleves, profs };
  }, [rows]);

  return (
    <ModulePageShell>
      <ModulePageHeader
        eyebrow="Vie scolaire"
        title="Absences déclarées à l’accueil"
        description="Élèves (et professeurs) signalés absents par le standard — source unique pour les absences élèves prévenues par téléphone, absentes des autres écrans vie scolaire. Consultation seule."
        actions={
          <button
            type="button"
            disabled={busy}
            onClick={() => void load()}
            className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-50"
          >
            {busy ? "Actualisation…" : "Actualiser"}
          </button>
        }
      />

      <div className="mb-6 flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <label className="text-sm">
          <span className="font-semibold text-slate-700">Date</span>
          <input
            type="date"
            value={date}
            onChange={(e) => setDate(e.target.value)}
            className="mt-1 block rounded-xl border border-slate-200 px-3 py-2"
          />
        </label>

        <div className="flex flex-wrap gap-2">
          {(
            [
              ["eleve", `Élèves (${counts.eleves})`],
              ["professeur", `Professeurs (${counts.profs})`],
              ["tous", `Tous (${counts.total})`],
            ] as const
          ).map(([value, label]) => (
            <button
              key={value}
              type="button"
              onClick={() => setKindFilter(value)}
              className={`rounded-full px-3 py-1.5 text-sm font-semibold ${
                kindFilter === value
                  ? "bg-indigo-600 text-white"
                  : "border border-slate-200 bg-white text-slate-700 hover:bg-slate-50"
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      <label className="mb-4 block">
        <span className="sr-only">Rechercher</span>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filtrer par nom, classe, motif…"
          className="w-full rounded-2xl border border-slate-200 px-4 py-3 text-base outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100"
        />
      </label>

      {error ? <p className="mb-4 text-sm font-medium text-rose-600">{error}</p> : null}

      <section className="rounded-3xl border border-slate-200 bg-white shadow-sm overflow-hidden">
        {busy && rows.length === 0 ? (
          <p className="px-6 py-12 text-center text-sm text-slate-500">Chargement…</p>
        ) : filtered.length === 0 ? (
          <p className="px-6 py-12 text-center text-sm text-slate-500">
            {rows.length === 0
              ? "Aucune absence déclarée à l’accueil pour cette date."
              : "Aucun résultat pour ce filtre."}
          </p>
        ) : (
          <ul className="divide-y divide-slate-100">
            {filtered.map((r) => (
              <li
                key={`${r.kind}-${r.id}`}
                className="flex flex-col gap-2 px-5 py-4 sm:flex-row sm:items-start sm:justify-between"
              >
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <span
                      className={`rounded-full px-2 py-0.5 text-[11px] font-bold uppercase ${kindBadgeClass(r.kind, r.eleveNature)}`}
                    >
                      {kindLabel(r.kind, r.eleveNature)}
                    </span>
                    <p className="font-semibold text-slate-900">{r.displayName}</p>
                  </div>
                  <p className="mt-1 text-sm text-slate-600">{r.subtitle}</p>
                  <p className="mt-0.5 text-xs text-slate-500">{formatPeriod(r)}</p>
                  {r.motif ? (
                    <p className="mt-1 text-sm text-slate-700">
                      <span className="font-medium">Motif :</span> {r.motif}
                    </p>
                  ) : null}
                </div>
                {r.createdByNom ? (
                  <p className="shrink-0 text-xs text-slate-500 sm:text-right">
                    Déclaré par {r.createdByNom}
                  </p>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </section>
    </ModulePageShell>
  );
}
