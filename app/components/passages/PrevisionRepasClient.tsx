"use client";

import { useCallback, useEffect, useState } from "react";
import {
  PREVISION_STATUT_LABELS,
  REPAS_SERVICE_LABELS,
  type PrevisionLigne,
  type PrevisionResume,
  type RepasService,
} from "@/app/lib/passages-prevision-shared";

function todayParis(): string {
  return new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

function normalizeIsoDate(raw: string | null): string | null {
  if (!raw) return null;
  const m = /^(\d{4}-\d{2}-\d{2})$/.exec(raw.trim());
  return m ? m[1]! : null;
}

function statutClass(statut: PrevisionLigne["statut"]): string {
  if (statut === "attendu_pris") return "border-emerald-200 bg-emerald-50 text-emerald-950";
  if (statut === "attendu_manquant") return "border-amber-200 bg-amber-50 text-amber-950";
  if (statut === "en_sortie") return "border-sky-200 bg-sky-50 text-sky-950";
  return "border-rose-200 bg-rose-50 text-rose-950";
}

export default function PrevisionRepasClient() {
  const [date, setDate] = useState(todayParis);
  const [service, setService] = useState<"" | RepasService>("");
  const [resume, setResume] = useState<PrevisionResume | null>(null);
  const [lignes, setLignes] = useState<PrevisionLigne[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    const params = new URLSearchParams({ prevision: "1", date });
    if (service) params.set("service", service);
    const res = await fetch(`/api/passages?${params.toString()}`, { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      error?: string;
      resume?: PrevisionResume;
      lignes?: PrevisionLigne[];
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      setResume(null);
      setLignes([]);
      setLoading(false);
      return;
    }
    setResume(data.resume ?? null);
    setLignes(data.lignes ?? []);
    setLoading(false);
  }, [date, service]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Jour</h2>
        <p className="mt-1 text-sm text-slate-600">
          Droit = grille repas (sinon régime). Pris = passage self. Sortie scolaire = hors attendus
          cantine. Week-end : pas de grille Lun–Ven.
        </p>
        <div className="mt-4 flex flex-wrap gap-3">
          <label className="text-sm font-semibold text-slate-800">
            Date
            <input
              type="date"
              value={date}
              lang="fr-CA"
              onChange={(e) => {
                const v = normalizeIsoDate(e.target.value);
                if (v) setDate(v);
              }}
              className="mt-1 block rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="text-sm font-semibold text-slate-800">
            Service
            <select
              value={service}
              onChange={(e) => setService(e.target.value as "" | RepasService)}
              className="mt-1 block rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            >
              <option value="">Midi + soir</option>
              <option value="midi">Midi</option>
              <option value="soir">Soir</option>
            </select>
          </label>
          <button
            type="button"
            onClick={() => void load()}
            className="self-end rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white"
          >
            Actualiser
          </button>
        </div>
      </section>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-900">
          {error}
        </p>
      ) : null}

      {resume ? (
        <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
          <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
            Synthèse
            {resume.jourLabel ? ` — ${resume.jourLabel}` : " — week-end"}
          </h2>
          {!resume.jourCle ? (
            <p className="mt-3 text-sm text-slate-600">
              Pas de prévision grille le week-end. Choisissez un jour ouvré.
            </p>
          ) : (
            <dl className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-5">
              <div className="rounded-2xl border border-slate-100 bg-slate-50 px-3 py-3">
                <dt className="text-xs font-medium text-slate-500">Attendus</dt>
                <dd className="mt-1 text-2xl font-bold text-slate-950">{resume.attendus}</dd>
              </div>
              <div className="rounded-2xl border border-emerald-100 bg-emerald-50/60 px-3 py-3">
                <dt className="text-xs font-medium text-emerald-800">Pris</dt>
                <dd className="mt-1 text-2xl font-bold text-emerald-950">{resume.pris}</dd>
              </div>
              <div className="rounded-2xl border border-amber-100 bg-amber-50/60 px-3 py-3">
                <dt className="text-xs font-medium text-amber-800">Manquants</dt>
                <dd className="mt-1 text-2xl font-bold text-amber-950">{resume.manquants}</dd>
              </div>
              <div className="rounded-2xl border border-rose-100 bg-rose-50/60 px-3 py-3">
                <dt className="text-xs font-medium text-rose-800">Imprévus</dt>
                <dd className="mt-1 text-2xl font-bold text-rose-950">{resume.imprevus}</dd>
              </div>
              <div className="rounded-2xl border border-sky-100 bg-sky-50/60 px-3 py-3">
                <dt className="text-xs font-medium text-sky-800">En sortie</dt>
                <dd className="mt-1 text-2xl font-bold text-sky-950">{resume.enSortie}</dd>
              </div>
            </dl>
          )}
        </section>
      ) : null}

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Détail</h2>
        {loading ? (
          <p className="mt-3 text-sm text-slate-500">Chargement…</p>
        ) : lignes.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">
            Aucune ligne (pas de droit ni de passage self pour ce filtre).
          </p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {lignes.map((l) => (
              <li
                key={`${l.eleveId}-${l.service}`}
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
                    {REPAS_SERVICE_LABELS[l.service]}
                    {l.sourceDroit ? ` · droit ${l.sourceDroit}` : ""}
                  </p>
                </div>
                <span
                  className={`rounded-full border px-2.5 py-1 text-xs font-bold ${statutClass(l.statut)}`}
                >
                  {PREVISION_STATUT_LABELS[l.statut]}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
