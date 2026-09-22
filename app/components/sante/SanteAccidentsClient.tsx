"use client";

import { useCallback, useEffect, useState } from "react";
import type { SanteAccidentRow } from "@/app/lib/sante-accidents-shared";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };

function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

export default function SanteAccidentsClient() {
  const [accidents, setAccidents] = useState<SanteAccidentRow[]>([]);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [dateAccident, setDateAccident] = useState(todayIso);
  const [circonstances, setCirconstances] = useState("");
  const [soins, setSoins] = useState("");
  const [suite, setSuite] = useState("");
  const [lieu, setLieu] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    const res = await fetch("/api/sante/accidents", { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      accidents?: SanteAccidentRow[];
      error?: string;
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      return;
    }
    setAccidents(data.accidents ?? []);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (q.trim().length < 2 || selected) {
      setHits([]);
      return;
    }
    const t = setTimeout(() => {
      void (async () => {
        const res = await fetch(`/api/sante/accidents?q=${encodeURIComponent(q.trim())}`, {
          credentials: "include",
        });
        const data = (await res.json().catch(() => ({}))) as { eleves?: EleveLite[] };
        if (res.ok) setHits(data.eleves ?? []);
      })();
    }, 220);
    return () => clearTimeout(t);
  }, [q, selected]);

  async function enregistrer() {
    if (!selected) {
      setError("Choisissez un élève.");
      return;
    }
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/sante/accidents", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          eleveId: selected.id,
          dateAccident,
          circonstances,
          soins,
          suite,
          lieu,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        setError(data.error || "Enregistrement impossible.");
        return;
      }
      setOkMsg(`Accident enregistré pour ${selected.prenom} ${selected.nom}.`);
      setCirconstances("");
      setSoins("");
      setSuite("");
      setLieu("");
      setDateAccident(todayIso());
      await load();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Nouvel accident
        </h2>
        <p className="mt-1 text-sm text-slate-600">
          Registre à conservation longue. Pas de suppression — on complète, on n’efface pas.
        </p>
        <div className="mt-4 space-y-3">
          <label className="block text-sm font-semibold text-slate-800">
            Élève
            <input
              value={selected ? `${selected.prenom} ${selected.nom}` : q}
              onChange={(e) => {
                setSelected(null);
                setQ(e.target.value);
              }}
              placeholder="Nom ou prénom"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          {!selected && hits.length > 0 ? (
            <ul className="overflow-hidden rounded-xl border border-slate-200">
              {hits.map((e) => (
                <li key={e.id}>
                  <button
                    type="button"
                    className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-slate-50"
                    onClick={() => {
                      setSelected(e);
                      setQ("");
                      setHits([]);
                    }}
                  >
                    <span className="font-semibold">
                      {e.prenom} {e.nom}
                    </span>
                    <span className="text-xs text-slate-500">{e.classe || "—"}</span>
                  </button>
                </li>
              ))}
            </ul>
          ) : null}
          <label className="block text-sm font-semibold text-slate-800">
            Date
            <input
              type="date"
              value={dateAccident}
              onChange={(e) => setDateAccident(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Lieu (optionnel)
            <input
              value={lieu}
              onChange={(e) => setLieu(e.target.value)}
              placeholder="Cour, gymnase…"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Circonstances
            <textarea
              value={circonstances}
              onChange={(e) => setCirconstances(e.target.value)}
              rows={2}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Soins
            <textarea
              value={soins}
              onChange={(e) => setSoins(e.target.value)}
              rows={2}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Suite
            <input
              value={suite}
              onChange={(e) => setSuite(e.target.value)}
              placeholder="Retour cours, famille prévenue…"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <button
            type="button"
            disabled={busy || !selected || !circonstances.trim()}
            onClick={() => void enregistrer()}
            className="rounded-xl bg-rose-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer l’accident
          </button>
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

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Registre récent
        </h2>
        {accidents.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucun accident enregistré.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {accidents.map((a) => (
              <li key={a.id} className="py-3">
                <p className="font-semibold text-slate-900">
                  {a.elevePrenom} {a.eleveNom}
                  <span className="ml-2 text-xs font-medium text-slate-500">{a.dateAccident}</span>
                </p>
                <p className="text-sm text-slate-700">{a.circonstances}</p>
                {a.soins ? (
                  <p className="text-xs text-slate-600">Soins : {a.soins}</p>
                ) : null}
                {a.suite ? (
                  <p className="text-xs text-slate-500">Suite : {a.suite}</p>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
