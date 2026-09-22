"use client";

import { useCallback, useEffect, useState } from "react";
import {
  CANTINE_FACTU_MODE_LABELS,
  type CantineFactuLigne,
  type CantineFactuMode,
  type CantineFactuResume,
} from "@/app/lib/passages-facturation-shared";

function todayParis(): string {
  return new Date().toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
}

function money(n: number): string {
  return n.toLocaleString("fr-FR", { style: "currency", currency: "EUR" });
}

export default function FacturationCantineClient() {
  const [debut, setDebut] = useState(todayParis);
  const [fin, setFin] = useState(todayParis);
  const [mode, setMode] = useState<CantineFactuMode>("forfait");
  const [modeSauve, setModeSauve] = useState<CantineFactuMode>("forfait");
  const [resume, setResume] = useState<CantineFactuResume | null>(null);
  const [lignes, setLignes] = useState<CantineFactuLigne[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    const params = new URLSearchParams({
      facturation: "1",
      debut,
      fin,
      mode,
    });
    const res = await fetch(`/api/passages?${params.toString()}`, { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      error?: string;
      resume?: CantineFactuResume;
      lignes?: CantineFactuLigne[];
      modeSauve?: CantineFactuMode;
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
    if (data.modeSauve) setModeSauve(data.modeSauve);
    setLoading(false);
  }, [debut, fin, mode]);

  useEffect(() => {
    void load();
  }, [load]);

  async function sauverMode() {
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/passages", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "setCantineFactuMode", mode }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        mode?: CantineFactuMode;
      };
      if (!res.ok) {
        setError(data.error || "Enregistrement impossible.");
        return;
      }
      if (data.mode) setModeSauve(data.mode);
      setOkMsg(`Réglage enregistré : ${CANTINE_FACTU_MODE_LABELS[mode]}.`);
      await load();
    } finally {
      setBusy(false);
    }
  }

  async function creerBrouillons() {
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/passages", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "createCantineBrouillons",
          mode,
          debut,
          fin,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        factures?: Array<{ numero: string; totalTtc: string }>;
        skippedSansFoyer?: number;
      };
      if (!res.ok) {
        setError(data.error || "Création impossible.");
        return;
      }
      const n = data.factures?.length ?? 0;
      const skip = data.skippedSansFoyer ?? 0;
      setOkMsg(
        n > 0
          ? `${n} facture(s) brouillon créée(s)${skip ? ` — ${skip} élève(s) sans foyer ignoré(s)` : ""}.`
          : skip
            ? `Aucun foyer lié — ${skip} élève(s) non facturés.`
            : "Aucune ligne à facturer.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Réglage</h2>
        <p className="mt-1 text-sm text-slate-600">
          Forfait = droit (grille / régime). Réel = passages self sur la période. Tarif unitaire{" "}
          <code className="text-xs">REPAS</code>.
        </p>
        <div className="mt-4 flex flex-wrap gap-3">
          <label className="text-sm font-semibold text-slate-800">
            Mode
            <select
              value={mode}
              onChange={(e) => setMode(e.target.value as CantineFactuMode)}
              className="mt-1 block rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            >
              <option value="forfait">{CANTINE_FACTU_MODE_LABELS.forfait}</option>
              <option value="reel">{CANTINE_FACTU_MODE_LABELS.reel}</option>
            </select>
          </label>
          <label className="text-sm font-semibold text-slate-800">
            Du
            <input
              type="date"
              value={debut}
              onChange={(e) => setDebut(e.target.value)}
              className="mt-1 block rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="text-sm font-semibold text-slate-800">
            Au
            <input
              type="date"
              value={fin}
              onChange={(e) => setFin(e.target.value)}
              className="mt-1 block rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <button
            type="button"
            disabled={busy}
            onClick={() => void sauverMode()}
            className="self-end rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer le mode
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="self-end rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-bold text-slate-800"
          >
            Actualiser
          </button>
        </div>
        <p className="mt-3 text-xs text-slate-500">
          Mode enregistré pour l’établissement :{" "}
          <span className="font-semibold">{CANTINE_FACTU_MODE_LABELS[modeSauve]}</span>
        </p>
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

      {resume ? (
        <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
          <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
            Synthèse — {resume.mode === "reel" ? "réel" : "forfait"}
          </h2>
          <dl className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-4">
            <div className="rounded-2xl border border-slate-100 bg-slate-50 px-3 py-3">
              <dt className="text-xs font-medium text-slate-500">Élèves</dt>
              <dd className="mt-1 text-2xl font-bold text-slate-950">{resume.eleves}</dd>
            </div>
            <div className="rounded-2xl border border-slate-100 bg-slate-50 px-3 py-3">
              <dt className="text-xs font-medium text-slate-500">Repas</dt>
              <dd className="mt-1 text-2xl font-bold text-slate-950">{resume.quantiteTotale}</dd>
            </div>
            <div className="rounded-2xl border border-emerald-100 bg-emerald-50/60 px-3 py-3">
              <dt className="text-xs font-medium text-emerald-800">Montant</dt>
              <dd className="mt-1 text-xl font-bold text-emerald-950">
                {money(resume.montantTotal)}
              </dd>
            </div>
            <div className="rounded-2xl border border-amber-100 bg-amber-50/60 px-3 py-3">
              <dt className="text-xs font-medium text-amber-800">Sans foyer</dt>
              <dd className="mt-1 text-2xl font-bold text-amber-950">{resume.sansFoyer}</dd>
            </div>
          </dl>
          <p className="mt-3 text-xs text-slate-500">
            Tarif {resume.tarifCode} · {money(resume.prixUnitaire)} / repas
          </p>
          <button
            type="button"
            disabled={busy || lignes.length === 0}
            onClick={() => void creerBrouillons()}
            className="mt-4 rounded-xl bg-indigo-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Créer brouillons (par foyer)
          </button>
        </section>
      ) : null}

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Lignes</h2>
        {loading ? (
          <p className="mt-3 text-sm text-slate-500">Chargement…</p>
        ) : lignes.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucune ligne pour cette période / ce mode.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {lignes.map((l) => (
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
                    {l.detail}
                    {!l.foyerId ? " · pas de foyer" : ""}
                  </p>
                </div>
                <span className="text-sm font-bold text-slate-900">
                  {l.quantite} × {money(l.prixUnitaire)} = {money(l.total)}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
