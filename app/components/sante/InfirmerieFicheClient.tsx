"use client";

import { useEffect, useState } from "react";
import type { InfirmerieFicheRow } from "@/app/lib/infirmerie-fiche-shared";

type EleveLite = { id: string; nom: string; prenom: string; classe: string | null };

export default function InfirmerieFicheClient() {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveLite[]>([]);
  const [selected, setSelected] = useState<EleveLite | null>(null);
  const [fiche, setFiche] = useState<InfirmerieFicheRow | null>(null);
  const [antecedents, setAntecedents] = useState("");
  const [personnes, setPersonnes] = useState("");
  const [notes, setNotes] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [okMsg, setOkMsg] = useState<string | null>(null);

  useEffect(() => {
    if (q.trim().length < 2 || selected) {
      setHits([]);
      return;
    }
    const t = setTimeout(() => {
      void (async () => {
        const res = await fetch(`/api/sante/fiches?q=${encodeURIComponent(q.trim())}`, {
          credentials: "include",
        });
        const data = (await res.json().catch(() => ({}))) as { eleves?: EleveLite[] };
        if (res.ok) setHits(data.eleves ?? []);
      })();
    }, 220);
    return () => clearTimeout(t);
  }, [q, selected]);

  async function loadFiche(eleve: EleveLite) {
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch(`/api/sante/fiches?eleveId=${encodeURIComponent(eleve.id)}`, {
        credentials: "include",
      });
      const data = (await res.json().catch(() => ({}))) as {
        fiche?: InfirmerieFicheRow;
        error?: string;
      };
      if (!res.ok) {
        setError(data.error || "Chargement impossible.");
        return;
      }
      const f = data.fiche!;
      setFiche(f);
      setAntecedents(f.antecedents);
      setPersonnes(f.personnesAPrevenir);
      setNotes(f.notes);
    } finally {
      setBusy(false);
    }
  }

  async function enregistrer() {
    if (!selected) {
      setError("Choisissez un élève.");
      return;
    }
    setBusy(true);
    setError(null);
    setOkMsg(null);
    try {
      const res = await fetch("/api/sante/fiches", {
        method: "PUT",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          eleveId: selected.id,
          antecedents,
          personnesAPrevenir: personnes,
          notes,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as {
        fiche?: InfirmerieFicheRow;
        error?: string;
      };
      if (!res.ok) {
        setError(data.error || "Enregistrement impossible.");
        return;
      }
      setFiche(data.fiche!);
      setOkMsg(`Fiche enregistrée pour ${selected.prenom} ${selected.nom}.`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
        <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">Élève</h2>
        <p className="mt-1 text-sm text-slate-600">
          Antécédents utiles à l’établissement et personnes à prévenir — pas le dossier du médecin
          scolaire.
        </p>
        <label className="mt-4 block text-sm font-semibold text-slate-800">
          Rechercher
          <input
            value={selected ? `${selected.prenom} ${selected.nom}` : q}
            onChange={(e) => {
              setSelected(null);
              setFiche(null);
              setQ(e.target.value);
            }}
            placeholder="Nom ou prénom"
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
          />
        </label>
        {!selected && hits.length > 0 ? (
          <ul className="mt-2 overflow-hidden rounded-xl border border-slate-200">
            {hits.map((e) => (
              <li key={e.id}>
                <button
                  type="button"
                  className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-slate-50"
                  onClick={() => {
                    setSelected(e);
                    setQ("");
                    setHits([]);
                    void loadFiche(e);
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

      {selected && fiche !== null ? (
        <section className="space-y-3 rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
          <h2 className="text-sm font-bold uppercase tracking-wide text-slate-500">
            Fiche — {selected.prenom} {selected.nom}
            {selected.classe ? (
              <span className="ml-2 text-xs font-medium normal-case text-slate-500">
                {selected.classe}
              </span>
            ) : null}
          </h2>
          <label className="block text-sm font-semibold text-slate-800">
            Antécédents utiles
            <textarea
              value={antecedents}
              onChange={(e) => setAntecedents(e.target.value)}
              rows={3}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              placeholder="Asthme, allergies connues de l’établissement…"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Personnes à prévenir
            <textarea
              value={personnes}
              onChange={(e) => setPersonnes(e.target.value)}
              rows={2}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              placeholder="Parent 1 — 06… ; Parent 2…"
            />
          </label>
          <label className="block text-sm font-semibold text-slate-800">
            Notes
            <textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              rows={2}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm font-normal"
              placeholder="Consignes infirmerie…"
            />
          </label>
          {fiche.updatedAt ? (
            <p className="text-xs text-slate-500">
              Dernière mise à jour {new Date(fiche.updatedAt).toLocaleString("fr-FR")}
            </p>
          ) : (
            <p className="text-xs text-slate-500">Aucune fiche enregistrée encore.</p>
          )}
          <button
            type="button"
            disabled={busy}
            onClick={() => void enregistrer()}
            className="rounded-xl bg-rose-700 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Enregistrer la fiche
          </button>
        </section>
      ) : null}
    </div>
  );
}
