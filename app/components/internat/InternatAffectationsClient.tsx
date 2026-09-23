"use client";

import { useCallback, useEffect, useState } from "react";
import type {
  InternatAffectationRow,
  InternatChambreRow,
} from "@/app/lib/internat-chambres-shared";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };

export default function InternatAffectationsClient() {
  const [chambres, setChambres] = useState<InternatChambreRow[]>([]);
  const [affectations, setAffectations] = useState<InternatAffectationRow[]>([]);
  const [batimentLabel, setBatimentLabel] = useState("Internat");
  const [chambreLabel, setChambreLabel] = useState("");
  const [capacite, setCapacite] = useState(2);
  const [chambreId, setChambreId] = useState("");
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setError(null);
    const res = await fetch("/api/internat/affectations", { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      error?: string;
      chambres?: InternatChambreRow[];
      affectations?: InternatAffectationRow[];
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      return;
    }
    setChambres(data.chambres ?? []);
    setAffectations(data.affectations ?? []);
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
        const res = await fetch(
          `/api/internat/affectations?q=${encodeURIComponent(q.trim())}`,
          { credentials: "include" },
        );
        const data = (await res.json().catch(() => ({}))) as { eleves?: EleveLite[] };
        if (res.ok) setHits(data.eleves ?? []);
      })();
    }, 220);
    return () => clearTimeout(t);
  }, [q, selected]);

  async function creerChambre() {
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/internat/affectations", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "createChambre",
          batimentLabel,
          label: chambreLabel,
          capacite,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        chambre?: InternatChambreRow;
      };
      if (!res.ok) {
        setError(data.error || "Création impossible.");
        return;
      }
      setOkMsg(`Chambre ${data.chambre?.label} créée.`);
      setChambreLabel("");
      if (data.chambre) setChambreId(data.chambre.id);
      await load();
    } finally {
      setBusy(false);
    }
  }

  async function affecter() {
    if (!selected || !chambreId) {
      setError("Choisissez une chambre et un élève.");
      return;
    }
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/internat/affectations", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "openAffectation",
          chambreId,
          eleveId: selected.id,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        error?: string;
        affectation?: InternatAffectationRow;
      };
      if (!res.ok) {
        setError(data.error || "Affectation impossible.");
        return;
      }
      const a = data.affectation;
      setOkMsg(
        a
          ? `${a.elevePrenom} ${a.eleveNom} → ${a.batimentLabel} / ${a.chambreLabel} depuis ${a.dateDebut}.`
          : "Affectation enregistrée.",
      );
      setSelected(null);
      setQ("");
      await load();
    } finally {
      setBusy(false);
    }
  }

  async function liberer(id: string) {
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/internat/affectations", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "closeAffectation", affectationId: id }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        setError(data.error || "Libération impossible.");
        return;
      }
      setOkMsg("Affectation fermée (date de fin renseignée).");
      await load();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <p className="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700">
        Postgres first : lits datés. Le hub JSON existant n’est ni lu ni effacé ici.{" "}
        <a href="/gestion-internat/appel-soir" className="font-bold text-indigo-700 hover:underline">
          Appel du soir
        </a>
        {" · "}
        <a href="/gestion-internat/sorties" className="font-bold text-indigo-700 hover:underline">
          Sorties week-end →
        </a>
      </p>

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Nouvelle chambre
        </h2>
        <div className="mt-4 grid gap-3 sm:grid-cols-3">
          <label className="text-sm font-semibold text-slate-800">
            Bâtiment
            <input
              value={batimentLabel}
              onChange={(e) => setBatimentLabel(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="text-sm font-semibold text-slate-800">
            Chambre
            <input
              value={chambreLabel}
              onChange={(e) => setChambreLabel(e.target.value)}
              placeholder="ex. 12"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="text-sm font-semibold text-slate-800">
            Capacité
            <input
              type="number"
              min={1}
              max={8}
              value={capacite}
              onChange={(e) => setCapacite(Number(e.target.value) || 2)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
        </div>
        <button
          type="button"
          disabled={busy || !chambreLabel.trim()}
          onClick={() => void creerChambre()}
          className="mt-4 rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
        >
          Créer la chambre
        </button>
      </section>

      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Affecter un élève
        </h2>
        <div className="mt-4 space-y-3">
          <label className="block text-sm font-semibold text-slate-800">
            Chambre
            <select
              value={chambreId}
              onChange={(e) => setChambreId(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            >
              <option value="">—</option>
              {chambres.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.batimentLabel} / {c.label} ({c.placesOccupees}/{c.capacite})
                </option>
              ))}
            </select>
          </label>
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
          <button
            type="button"
            disabled={busy || !selected || !chambreId}
            onClick={() => void affecter()}
            className="rounded-xl bg-indigo-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Affecter (lit ouvert)
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
          Affectations ouvertes
        </h2>
        {affectations.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucune affectation ouverte.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {affectations.map((a) => (
              <li
                key={a.id}
                className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm"
              >
                <div>
                  <p className="font-semibold text-slate-900">
                    {a.elevePrenom} {a.eleveNom}
                    {a.eleveClasse ? (
                      <span className="ml-2 text-xs font-medium text-slate-500">
                        {a.eleveClasse}
                      </span>
                    ) : null}
                  </p>
                  <p className="text-xs text-slate-500">
                    {a.batimentLabel} / {a.chambreLabel} · depuis {a.dateDebut}
                  </p>
                </div>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void liberer(a.id)}
                  className="rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-bold text-slate-700 hover:bg-slate-50"
                >
                  Libérer
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
