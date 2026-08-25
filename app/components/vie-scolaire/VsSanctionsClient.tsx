"use client";

import { useCallback, useEffect, useState } from "react";

type SanctionType = { id: string; code: string; libelle: string; gravite: number };
type EleveHit = { id: string; nom: string; prenom: string; classe: string | null };
type SanctionRow = {
  id: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  typeLibelle: string;
  dateSanction: string;
  motif: string | null;
  createdByNom: string | null;
};

function todayIso(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export default function VsSanctionsClient() {
  const [types, setTypes] = useState<SanctionType[]>([]);
  const [sanctions, setSanctions] = useState<SanctionRow[]>([]);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveHit[]>([]);
  const [eleveId, setEleveId] = useState("");
  const [eleveLabel, setEleveLabel] = useState("");
  const [typeId, setTypeId] = useState("");
  const [dateSanction, setDateSanction] = useState(todayIso());
  const [motif, setMotif] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/vie-scolaire/sanctions", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setTypes(data.types || []);
      setSanctions(data.sanctions || []);
      setTypeId((prev) => prev || data.types?.[0]?.id || "");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (q.trim().length < 2) {
      setHits([]);
      return;
    }
    const t = setTimeout(() => {
      void (async () => {
        const res = await fetch(`/api/vie-scolaire/sanctions?q=${encodeURIComponent(q)}`, {
          cache: "no-store",
        });
        const data = await res.json();
        if (res.ok) setHits(data.eleves || []);
      })();
    }, 250);
    return () => clearTimeout(t);
  }, [q]);

  const submit = async () => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/vie-scolaire/sanctions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ eleveId, typeId, dateSanction, motif }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Création impossible");
      setMessage("Sanction enregistrée.");
      setMotif("");
      setEleveId("");
      setEleveLabel("");
      setQ("");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const annuler = async (id: string) => {
    setBusy(true);
    try {
      const res = await fetch("/api/vie-scolaire/sanctions", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "annuler", id }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Annulation impossible");
      setMessage("Sanction annulée.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="max-w-3xl mx-auto px-4 py-6 space-y-6">
      <header>
        <h1 className="text-2xl font-black text-slate-900">Sanctions</h1>
        <p className="text-sm text-slate-600 mt-1">
          Catalogue court — avertissement, colle, exclusion de cours, blâme. Pas de permis à points.
        </p>
      </header>

      <section className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
        <h2 className="font-bold text-slate-900">Nouvelle sanction</h2>
        <label className="block text-sm font-semibold">
          Élève
          <input
            value={eleveLabel || q}
            onChange={(e) => {
              setEleveId("");
              setEleveLabel("");
              setQ(e.target.value);
            }}
            placeholder="Nom, prénom ou classe…"
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
          />
        </label>
        {hits.length > 0 && !eleveId && (
          <ul className="rounded-xl border border-slate-100 divide-y max-h-40 overflow-y-auto">
            {hits.map((h) => (
              <li key={h.id}>
                <button
                  type="button"
                  className="w-full text-left px-3 py-2 text-sm hover:bg-indigo-50"
                  onClick={() => {
                    setEleveId(h.id);
                    setEleveLabel(`${h.prenom} ${h.nom}${h.classe ? ` (${h.classe})` : ""}`);
                    setHits([]);
                    setQ("");
                  }}
                >
                  {h.prenom} {h.nom}
                  {h.classe ? ` · ${h.classe}` : ""}
                </button>
              </li>
            ))}
          </ul>
        )}
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="block text-sm font-semibold">
            Type
            <select
              value={typeId}
              onChange={(e) => setTypeId(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
            >
              {types.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.libelle}
                </option>
              ))}
            </select>
          </label>
          <label className="block text-sm font-semibold">
            Date
            <input
              type="date"
              value={dateSanction}
              onChange={(e) => setDateSanction(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
            />
          </label>
        </div>
        <label className="block text-sm font-semibold">
          Motif
          <textarea
            value={motif}
            onChange={(e) => setMotif(e.target.value)}
            rows={2}
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
            placeholder="Faits succincts…"
          />
        </label>
        <button
          type="button"
          disabled={busy || !eleveId || !typeId}
          onClick={() => void submit()}
          className="rounded-xl bg-indigo-600 text-white px-4 py-2 font-bold disabled:opacity-50"
        >
          Enregistrer
        </button>
      </section>

      <section className="space-y-3">
        <h2 className="font-bold text-slate-900">Sanctions actives</h2>
        {!sanctions.length && !busy && (
          <p className="text-sm text-slate-600 rounded-2xl border border-slate-200 bg-white p-4">
            Aucune sanction active.
          </p>
        )}
        <ul className="space-y-2">
          {sanctions.map((s) => (
            <li
              key={s.id}
              className="rounded-2xl border border-slate-200 bg-white p-4 flex flex-wrap justify-between gap-3"
            >
              <div>
                <p className="font-bold text-slate-900">
                  {s.elevePrenom} {s.eleveNom}
                  {s.eleveClasse ? ` · ${s.eleveClasse}` : ""}
                </p>
                <p className="text-sm text-slate-600">
                  {s.typeLibelle} ·{" "}
                  {s.dateSanction ? new Date(s.dateSanction).toLocaleDateString("fr-FR") : "—"}
                  {s.createdByNom ? ` · ${s.createdByNom}` : ""}
                </p>
                {s.motif && <p className="text-xs text-slate-500 mt-1">{s.motif}</p>}
              </div>
              <button
                type="button"
                disabled={busy}
                onClick={() => void annuler(s.id)}
                className="rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-bold disabled:opacity-50"
              >
                Annuler
              </button>
            </li>
          ))}
        </ul>
      </section>

      {message && (
        <p className="text-sm text-emerald-800 bg-emerald-50 border border-emerald-100 rounded-xl px-3 py-2">
          {message}
        </p>
      )}
      {error && <p className="text-sm text-red-600 font-semibold">{error}</p>}
    </div>
  );
}
