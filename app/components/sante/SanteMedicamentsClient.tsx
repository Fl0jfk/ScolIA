"use client";

import { useCallback, useEffect, useState } from "react";
import type { SanteMedicamentPriseRow } from "@/app/lib/sante-medicaments-shared";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };

function formatQuand(iso: string): string {
  try {
    return new Date(iso).toLocaleString("fr-FR", {
      day: "2-digit",
      month: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

export default function SanteMedicamentsClient() {
  const [prises, setPrises] = useState<SanteMedicamentPriseRow[]>([]);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [medicament, setMedicament] = useState("");
  const [dose, setDose] = useState("");
  const [notes, setNotes] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);

  const load = useCallback(async (eleveId?: string) => {
    setError(null);
    const params = new URLSearchParams();
    if (eleveId) params.set("eleveId", eleveId);
    const res = await fetch(`/api/sante/medicaments?${params}`, { credentials: "include" });
    const data = (await res.json().catch(() => ({}))) as {
      prises?: SanteMedicamentPriseRow[];
      error?: string;
    };
    if (!res.ok) {
      setError(data.error || "Chargement impossible.");
      return;
    }
    setPrises(data.prises ?? []);
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
        const res = await fetch(`/api/sante/medicaments?q=${encodeURIComponent(q.trim())}`, {
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
      const res = await fetch("/api/sante/medicaments", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          eleveId: selected.id,
          medicament,
          dose,
          notes,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string };
      if (!res.ok) {
        setError(data.error || "Enregistrement impossible.");
        return;
      }
      setOkMsg(`Prise enregistrée pour ${selected.prenom} ${selected.nom}.`);
      setMedicament("");
      setDose("");
      setNotes("");
      await load(selected.id);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
          Nouvelle prise
        </h2>
        <p className="mt-1 text-sm text-slate-600">
          Journal des prises à l’infirmerie. L’ordonnance reste un document dans le dossier.
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
                      void load(e.id);
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
            Médicament
            <input
              value={medicament}
              onChange={(e) => setMedicament(e.target.value)}
              placeholder="Ex. paracétamol"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Dose (optionnel)
            <input
              value={dose}
              onChange={(e) => setDose(e.target.value)}
              placeholder="Ex. 500 mg"
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Notes (optionnel)
            <input
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
            />
          </label>
          <button
            type="button"
            disabled={busy || !selected || !medicament.trim()}
            onClick={() => void enregistrer()}
            className="rounded-xl bg-rose-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer la prise
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
          Journal des prises
          {selected ? (
            <span className="ml-2 text-xs font-medium normal-case text-slate-500">
              — {selected.prenom} {selected.nom}
            </span>
          ) : (
            <span className="ml-2 text-xs font-medium normal-case text-slate-500">
              — récents
            </span>
          )}
        </h2>
        {prises.length === 0 ? (
          <p className="mt-3 text-sm text-slate-600">Aucune prise enregistrée.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {prises.map((p) => (
              <li key={p.id} className="py-3">
                <p className="font-semibold text-slate-900">
                  {p.elevePrenom} {p.eleveNom}
                  <span className="ml-2 text-xs font-medium text-slate-500">
                    {formatQuand(p.prisAt)}
                  </span>
                </p>
                <p className="text-sm text-slate-700">
                  {p.medicament}
                  {p.dose ? ` · ${p.dose}` : ""}
                </p>
                {p.notes ? <p className="text-xs text-slate-500">{p.notes}</p> : null}
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
