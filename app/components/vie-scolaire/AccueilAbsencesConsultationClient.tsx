"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { parisDateKey } from "@/app/lib/paris-time";
import {
  cycleLabel,
  type AccueilBoardKind,
  type AccueilBoardRow,
} from "@/app/lib/accueil-absences-types";

type KindFilter = "tous" | AccueilBoardKind;
type CycleFilter = "tous" | "ecole" | "college" | "lycee";

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

function shiftDateIso(iso: string, deltaDays: number): string {
  const [y, m, d] = iso.split("-").map(Number);
  const dt = new Date(Date.UTC(y!, m! - 1, d!));
  dt.setUTCDate(dt.getUTCDate() + deltaDays);
  return `${dt.getUTCFullYear()}-${String(dt.getUTCMonth() + 1).padStart(2, "0")}-${String(dt.getUTCDate()).padStart(2, "0")}`;
}

function formatDayTitleFr(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  const dt = new Date(Date.UTC(y!, m! - 1, d!, 12));
  return new Intl.DateTimeFormat("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(dt);
}

export default function AccueilAbsencesConsultationClient() {
  const today = parisDateKey(new Date());
  const [date, setDate] = useState(today);
  const [kindFilter, setKindFilter] = useState<KindFilter>("tous");
  const [cycleFilter, setCycleFilter] = useState<CycleFilter>("tous");
  const [classeFilter, setClasseFilter] = useState("tous");
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<AccueilBoardRow[]>([]);
  const [busy, setBusy] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

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

  useEffect(() => {
    setClasseFilter("tous");
  }, [cycleFilter, date]);

  const classeOptions = useMemo(() => {
    const set = new Set<string>();
    for (const r of rows) {
      if (r.kind !== "eleve") continue;
      if (cycleFilter !== "tous" && r.cycle !== cycleFilter) continue;
      const c = (r.classe || "").trim();
      if (c) set.add(c);
    }
    return [...set].sort((a, b) => a.localeCompare(b, "fr"));
  }, [rows, cycleFilter]);

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

    // Niveau / classe : ciblent les élèves (CPE collège / lycée / école).
    if (cycleFilter !== "tous" || classeFilter !== "tous") {
      list = list.filter((r) => r.kind === "eleve");
      if (cycleFilter !== "tous") {
        list = list.filter((r) => r.cycle === cycleFilter);
      }
      if (classeFilter !== "tous") {
        list = list.filter((r) => (r.classe || "").trim() === classeFilter);
      }
    }

    if (!needle) return list;
    return list.filter((r) => {
      const hay = `${r.displayName} ${r.subtitle} ${r.motif || ""} ${r.classe || ""}`
        .toLowerCase()
        .normalize("NFD")
        .replace(/[\u0300-\u036f]/g, "");
      return hay.includes(needle);
    });
  }, [rows, kindFilter, cycleFilter, classeFilter, query]);

  const counts = useMemo(() => {
    const eleves = rows.filter((r) => r.kind === "eleve").length;
    const profs = rows.filter((r) => r.kind === "professeur").length;
    return { total: rows.length, eleves, profs };
  }, [rows]);

  const supprimer = async (row: AccueilBoardRow) => {
    if (row.kind !== "eleve") return;
    const ok = window.confirm(
      `Supprimer la déclaration de ${row.displayName} ?\nElle disparaîtra de la liste (erreur de saisie).`,
    );
    if (!ok) return;
    setDeletingId(row.id);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/vie-scolaire/absences-accueil", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "annuler", id: row.id }),
      });
      const data = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(data.error || "Suppression impossible");
      setMessage(`Déclaration de ${row.displayName} supprimée.`);
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setDeletingId(null);
    }
  };

  const selectClassName =
    "mt-1 block w-full min-w-[10rem] rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm text-slate-800 outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100";

  return (
    <ModulePageShell>
      <ModulePageHeader
        eyebrow="Vie scolaire"
        title="Absences déclarées à l’accueil"
        description="Élèves (et professeurs) signalés absents par le standard — source unique pour les absences élèves prévenues par téléphone. Filtrez par niveau ou classe ; corrigez une erreur de saisie si besoin."
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

      <div className="mb-6 space-y-4">
        <div className="flex flex-col gap-3 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wide text-slate-500">Jour consulté</p>
            <p className="mt-0.5 text-lg font-semibold capitalize text-slate-900">
              {formatDayTitleFr(date)}
            </p>
            {date === today ? (
              <p className="text-xs font-medium text-emerald-700">Aujourd’hui</p>
            ) : null}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              onClick={() => setDate((d) => shiftDateIso(d, -1))}
              className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-50"
              aria-label="Jour précédent"
            >
              ←
            </button>
            <button
              type="button"
              disabled={date === today}
              onClick={() => setDate(today)}
              className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-40"
            >
              Aujourd’hui
            </button>
            <button
              type="button"
              onClick={() => setDate((d) => shiftDateIso(d, 1))}
              className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-50"
              aria-label="Jour suivant"
            >
              →
            </button>
            <label className="text-sm">
              <span className="sr-only">Choisir une date</span>
              <input
                type="date"
                value={date}
                onChange={(e) => {
                  if (e.target.value) setDate(e.target.value);
                }}
                className="rounded-xl border border-slate-200 px-3 py-2 text-sm"
              />
            </label>
          </div>
        </div>

        <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-end">
          <div className="flex flex-wrap gap-2">
            {(
              [
                ["tous", `Tous (${counts.total})`],
                ["eleve", `Élèves (${counts.eleves})`],
                ["professeur", `Professeurs (${counts.profs})`],
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

          <label className="text-sm">
            <span className="font-semibold text-slate-700">Niveau</span>
            <select
              value={cycleFilter}
              onChange={(e) => setCycleFilter(e.target.value as CycleFilter)}
              className={selectClassName}
            >
              <option value="tous">Tous les niveaux</option>
              <option value="ecole">{cycleLabel("ecole")}</option>
              <option value="college">{cycleLabel("college")}</option>
              <option value="lycee">{cycleLabel("lycee")}</option>
            </select>
          </label>

          <label className="text-sm">
            <span className="font-semibold text-slate-700">Classe</span>
            <select
              value={classeFilter}
              onChange={(e) => setClasseFilter(e.target.value)}
              className={selectClassName}
            >
              <option value="tous">Toutes les classes</option>
              {classeOptions.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </label>
        </div>

        <label className="block">
          <span className="sr-only">Rechercher</span>
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Rechercher un nom, motif…"
            className="w-full rounded-2xl border border-slate-200 px-4 py-3 text-base outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100"
          />
        </label>
      </div>

      {error ? <p className="mb-4 text-sm font-medium text-rose-600">{error}</p> : null}
      {message ? <p className="mb-4 text-sm font-medium text-emerald-700">{message}</p> : null}

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
                    {r.kind === "eleve" && r.cycle ? (
                      <span className="rounded-full bg-slate-100 px-2 py-0.5 text-[11px] font-semibold text-slate-600">
                        {cycleLabel(r.cycle)}
                      </span>
                    ) : null}
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
                <div className="flex shrink-0 flex-col items-stretch gap-2 sm:items-end">
                  {r.createdByNom ? (
                    <p className="text-xs text-slate-500 sm:text-right">
                      Déclaré par {r.createdByNom}
                    </p>
                  ) : null}
                  {r.kind === "eleve" ? (
                    <button
                      type="button"
                      disabled={busy || deletingId === r.id}
                      onClick={() => void supprimer(r)}
                      className="rounded-lg border border-rose-200 bg-white px-3 py-1.5 text-xs font-semibold text-rose-700 hover:bg-rose-50 disabled:opacity-50"
                    >
                      {deletingId === r.id ? "Suppression…" : "Supprimer"}
                    </button>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </ModulePageShell>
  );
}
