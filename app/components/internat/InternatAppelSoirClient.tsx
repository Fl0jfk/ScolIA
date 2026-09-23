"use client";

import { useCallback, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import {
  INTERNAT_APPEL_MARQUE_LABELS,
  INTERNAT_APPEL_MARQUES,
  type InternatAppelLigneRow,
  type InternatAppelMarque,
  type InternatAppelResume,
} from "@/app/lib/internat-appel-shared";

function todayParis(): string {
  return new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

function normalizeIsoDate(raw: string | null): string | null {
  if (!raw) return null;
  const m = /^(\d{4}-\d{2}-\d{2})$/.exec(raw.trim());
  return m ? m[1]! : null;
}

function marqueClass(m: InternatAppelMarque): string {
  if (m === "present") return "border-emerald-200 bg-emerald-50 text-emerald-950";
  if (m === "absent") return "border-rose-200 bg-rose-50 text-rose-950";
  if (m === "excuse") return "border-amber-200 bg-amber-50 text-amber-950";
  return "border-sky-200 bg-sky-50 text-sky-950";
}

export default function InternatAppelSoirClient() {
  const searchParams = useSearchParams();
  const dateFromUrl = normalizeIsoDate(searchParams.get("date"));
  const [date, setDate] = useState(dateFromUrl ?? todayParis);
  const [appel, setAppel] = useState<InternatAppelResume | null>(null);
  const [lignes, setLignes] = useState<InternatAppelLigneRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (dateFromUrl && dateFromUrl !== date) setDate(dateFromUrl);
  }, [dateFromUrl, date]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    const res = await fetch(`/api/internat/appel-soir?date=${encodeURIComponent(date)}`, {
      credentials: "include",
    });
    const data = (await res.json().catch(() => ({}))) as {
      error?: string;
      appel?: InternatAppelResume;
      lignes?: InternatAppelLigneRow[];
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      setAppel(null);
      setLignes([]);
      setLoading(false);
      return;
    }
    setAppel(data.appel ?? null);
    setLignes(data.lignes ?? []);
    setLoading(false);
  }, [date]);

  useEffect(() => {
    void load();
  }, [load]);

  async function setMarque(eleveId: string, marque: InternatAppelMarque) {
    if (!appel || appel.statut !== "ouverte") return;
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/internat/appel-soir", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "setMarque",
          appelId: appel.id,
          eleveId,
          marque,
          date,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        appel?: InternatAppelResume;
        lignes?: InternatAppelLigneRow[];
      };
      if (!res.ok) {
        setError(data.error || "Marquage impossible.");
        return;
      }
      if (data.appel) setAppel(data.appel);
      if (data.lignes) setLignes(data.lignes);
    } finally {
      setBusy(false);
    }
  }

  async function valider() {
    if (!appel) return;
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/internat/appel-soir", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "valider", appelId: appel.id }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        appel?: InternatAppelResume;
        lignes?: InternatAppelLigneRow[];
      };
      if (!res.ok) {
        setError(data.error || "Validation impossible.");
        return;
      }
      if (data.appel) setAppel(data.appel);
      if (data.lignes) setLignes(data.lignes);
      setOkMsg("Appel du soir validé.");
    } finally {
      setBusy(false);
    }
  }

  const locked = appel?.statut === "validee";

  return (
    <div className="space-y-6">
      <p className="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700">
        Roster = lits datés Postgres du jour. L’appel JSON du hub n’est ni lu ni modifié.
      </p>

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Jour</h2>
        <div className="mt-4 flex flex-wrap gap-3">
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
            className="self-end rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-bold text-slate-800"
          >
            Actualiser
          </button>
          {appel && !locked ? (
            <button
              type="button"
              disabled={busy || lignes.length === 0}
              onClick={() => void valider()}
              className="self-end rounded-xl bg-indigo-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
            >
              Valider l’appel
            </button>
          ) : null}
        </div>
      </section>

      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-900">
          {error}
        </p>
      ) : null}
      {okMsg ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-900">
          {okMsg}
        </p>
      ) : null}

      {appel ? (
        <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
          <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
            Synthèse — {appel.statut === "validee" ? "validé" : "ouvert"}
          </h2>
          <dl className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-5">
            <div className="rounded-2xl border border-slate-100 bg-slate-50 px-3 py-3">
              <dt className="text-xs font-medium text-slate-500">Effectif</dt>
              <dd className="mt-1 text-2xl font-bold text-slate-950">{appel.total}</dd>
            </div>
            <div className="rounded-2xl border border-emerald-100 bg-emerald-50/60 px-3 py-3">
              <dt className="text-xs font-medium text-emerald-800">Présents</dt>
              <dd className="mt-1 text-2xl font-bold text-emerald-950">{appel.presents}</dd>
            </div>
            <div className="rounded-2xl border border-rose-100 bg-rose-50/60 px-3 py-3">
              <dt className="text-xs font-medium text-rose-800">Absents</dt>
              <dd className="mt-1 text-2xl font-bold text-rose-950">{appel.absents}</dd>
            </div>
            <div className="rounded-2xl border border-amber-100 bg-amber-50/60 px-3 py-3">
              <dt className="text-xs font-medium text-amber-800">Excusés</dt>
              <dd className="mt-1 text-2xl font-bold text-amber-950">{appel.excuses}</dd>
            </div>
            <div className="rounded-2xl border border-sky-100 bg-sky-50/60 px-3 py-3">
              <dt className="text-xs font-medium text-sky-800">Activité</dt>
              <dd className="mt-1 text-2xl font-bold text-sky-950">{appel.activites}</dd>
            </div>
          </dl>
        </section>
      ) : null}

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Élèves</h2>
        {loading ? (
          <p className="mt-3 text-sm text-slate-500">Chargement…</p>
        ) : lignes.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">
            Aucun lit daté pour ce jour. Affectez d’abord des élèves.
          </p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {lignes.map((l) => (
              <li key={l.eleveId} className="space-y-2 py-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <p className="text-sm font-semibold text-slate-900">
                      {l.elevePrenom} {l.eleveNom}
                      {l.eleveClasse ? (
                        <span className="ml-2 text-xs font-medium text-slate-500">
                          {l.eleveClasse}
                        </span>
                      ) : null}
                    </p>
                    <p className="text-xs text-slate-500">
                      {l.batimentLabel && l.chambreLabel
                        ? `${l.batimentLabel} / ${l.chambreLabel}`
                        : "—"}
                    </p>
                  </div>
                  <span
                    className={`rounded-full border px-2.5 py-1 text-xs font-bold ${marqueClass(l.marque)}`}
                  >
                    {INTERNAT_APPEL_MARQUE_LABELS[l.marque]}
                  </span>
                </div>
                {!locked ? (
                  <div className="flex flex-wrap gap-2">
                    {INTERNAT_APPEL_MARQUES.map((m) => (
                      <button
                        key={m}
                        type="button"
                        disabled={busy || l.marque === m}
                        onClick={() => void setMarque(l.eleveId, m)}
                        className={`rounded-lg border px-2.5 py-1 text-xs font-bold disabled:opacity-40 ${
                          l.marque === m
                            ? marqueClass(m)
                            : "border-slate-200 bg-white text-slate-700 hover:bg-slate-50"
                        }`}
                      >
                        {INTERNAT_APPEL_MARQUE_LABELS[m]}
                      </button>
                    ))}
                  </div>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
