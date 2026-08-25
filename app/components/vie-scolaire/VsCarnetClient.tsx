"use client";

import { useCallback, useEffect, useState } from "react";

type Category = { id: string; label: string };
type EleveHit = { id: string; nom: string; prenom: string; classe: string | null };
type EntreeRow = {
  id: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateEntree: string;
  categorie: string;
  titre: string;
  corps: string;
  createdByNom: string | null;
  signeAt: string | null;
  signeParNom: string | null;
};

function todayIso(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function categorieLabel(categories: Category[], id: string): string {
  return categories.find((c) => c.id === id)?.label || id;
}

export default function VsCarnetClient() {
  const [categories, setCategories] = useState<Category[]>([]);
  const [entrees, setEntrees] = useState<EntreeRow[]>([]);
  const [filterNonSignees, setFilterNonSignees] = useState(false);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<EleveHit[]>([]);
  const [eleveId, setEleveId] = useState("");
  const [eleveLabel, setEleveLabel] = useState("");
  const [categorie, setCategorie] = useState("correspondance");
  const [dateEntree, setDateEntree] = useState(todayIso());
  const [titre, setTitre] = useState("");
  const [corps, setCorps] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const qs = filterNonSignees ? "?nonSignees=1" : "";
      const res = await fetch(`/api/vie-scolaire/carnet${qs}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setCategories(data.categories || []);
      setEntrees(data.entrees || []);
      setCategorie((prev) => prev || data.categories?.[0]?.id || "correspondance");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, [filterNonSignees]);

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
        const res = await fetch(`/api/vie-scolaire/carnet?q=${encodeURIComponent(q)}`, {
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
      const res = await fetch("/api/vie-scolaire/carnet", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ eleveId, dateEntree, categorie, titre, corps }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Création impossible");
      setMessage("Entrée publiée dans le carnet.");
      setTitre("");
      setCorps("");
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

  return (
    <div className="max-w-3xl mx-auto px-4 py-6 space-y-6">
      <header>
        <h1 className="text-2xl font-black text-slate-900">Carnet de correspondance</h1>
        <p className="text-sm text-slate-600 mt-1">
          Canal établissement → famille. La famille accuse lecture sur le portail — pas une messagerie.
        </p>
      </header>

      <section className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3">
        <h2 className="font-bold text-slate-900">Nouvelle entrée</h2>
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
            Date
            <input
              type="date"
              value={dateEntree}
              onChange={(e) => setDateEntree(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
            />
          </label>
          <label className="block text-sm font-semibold">
            Catégorie
            <select
              value={categorie}
              onChange={(e) => setCategorie(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
            >
              {categories.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        <label className="block text-sm font-semibold">
          Titre
          <input
            value={titre}
            onChange={(e) => setTitre(e.target.value)}
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
            maxLength={120}
          />
        </label>
        <label className="block text-sm font-semibold">
          Message
          <textarea
            value={corps}
            onChange={(e) => setCorps(e.target.value)}
            rows={4}
            className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
            maxLength={4000}
          />
        </label>
        <button
          type="button"
          disabled={busy || !eleveId || !titre.trim() || !corps.trim()}
          onClick={() => void submit()}
          className="bg-indigo-600 text-white px-5 py-2.5 rounded-xl font-bold text-sm disabled:opacity-40"
        >
          Publier
        </button>
      </section>

      {error && (
        <div className="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
          {error}
        </div>
      )}
      {message && (
        <div className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
          {message}
        </div>
      )}

      <section className="space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="font-bold text-slate-900">Entrées récentes</h2>
          <label className="flex items-center gap-2 text-xs font-semibold text-slate-600">
            <input
              type="checkbox"
              checked={filterNonSignees}
              onChange={(e) => setFilterNonSignees(e.target.checked)}
            />
            Non signées seulement
          </label>
        </div>
        {entrees.length === 0 ? (
          <p className="text-sm text-slate-500">Aucune entrée pour le moment.</p>
        ) : (
          <ul className="space-y-2">
            {entrees.map((e) => (
              <li key={e.id} className="rounded-2xl border border-slate-200 bg-white p-4">
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <div>
                    <p className="font-bold text-slate-900">
                      {e.elevePrenom} {e.eleveNom}
                      {e.eleveClasse ? (
                        <span className="ml-2 font-normal text-slate-500">{e.eleveClasse}</span>
                      ) : null}
                    </p>
                    <p className="text-xs text-slate-500 mt-0.5">
                      {e.dateEntree ? new Date(e.dateEntree).toLocaleDateString("fr-FR") : "—"} ·{" "}
                      {categorieLabel(categories, e.categorie)}
                      {e.createdByNom ? ` · ${e.createdByNom}` : ""}
                    </p>
                  </div>
                  <span
                    className={`rounded-full px-2.5 py-0.5 text-[11px] font-bold ${
                      e.signeAt
                        ? "bg-emerald-50 text-emerald-800"
                        : "bg-amber-50 text-amber-900"
                    }`}
                  >
                    {e.signeAt ? "Signé" : "En attente"}
                  </span>
                </div>
                <p className="mt-2 text-sm font-semibold text-slate-800">{e.titre}</p>
                <p className="mt-1 text-sm text-slate-600 whitespace-pre-wrap">{e.corps}</p>
                {e.signeAt ? (
                  <p className="mt-2 text-xs text-slate-400">
                    Accusé le {new Date(e.signeAt).toLocaleString("fr-FR")}
                    {e.signeParNom ? ` par ${e.signeParNom}` : ""}
                  </p>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
