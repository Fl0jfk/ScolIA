"use client";

import { useCallback, useEffect, useState } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";

type Entry = {
  id: string;
  dateSeance: string;
  classe: string | null;
  matiereLibelle: string | null;
  contenu: string;
  travail: string;
  aRendreLe: string | null;
  enseignantNom: string | null;
  visibleFamille: boolean;
};

function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

export default function CahierTexteStaffClient() {
  const [entries, setEntries] = useState<Entry[]>([]);
  const [classes, setClasses] = useState<string[]>([]);
  const [filterClasse, setFilterClasse] = useState("");
  const [dateSeance, setDateSeance] = useState(todayIso());
  const [classe, setClasse] = useState("");
  const [matiereLibelle, setMatiereLibelle] = useState("");
  const [contenu, setContenu] = useState("");
  const [travail, setTravail] = useState("");
  const [aRendreLe, setARendreLe] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const qs = new URLSearchParams();
      if (filterClasse) qs.set("classe", filterClasse);
      const res = await fetch(`/api/cahier-texte?${qs}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setEntries(data.entries || []);
      setClasses(data.classes || []);
      setClasse((prev) => prev || data.classes?.[0] || "");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, [filterClasse]);

  useEffect(() => {
    void load();
  }, [load]);

  const create = async () => {
    setBusy(true);
    setError(null);
    setOk(null);
    try {
      const res = await fetch("/api/cahier-texte", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          dateSeance,
          classe,
          matiereLibelle,
          contenu,
          travail,
          aRendreLe: aRendreLe || null,
          visibleFamille: true,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Enregistrement impossible");
      setContenu("");
      setTravail("");
      setARendreLe("");
      setMatiereLibelle("");
      setOk("Séance enregistrée — visible familles.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const hide = async (id: string) => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/cahier-texte", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "hide", id }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Action impossible");
      setOk("Entrée masquée aux familles.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Enseignement"
        title="Cahier de textes"
        description="Leçon du jour et travail à faire — visible sur le portail familles. Distinct des devoirs notés."
      />

      {error ? <p className="mb-3 text-sm font-semibold text-rose-700">{error}</p> : null}
      {ok ? <p className="mb-3 text-sm font-semibold text-emerald-700">{ok}</p> : null}

      <div className="grid gap-4 lg:grid-cols-2">
        <section className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3 text-sm">
          <h2 className="font-bold text-slate-900">Nouvelle séance</h2>
          <label className={`block ${dash.textMid}`}>
            Date
            <input
              type="date"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={dateSeance}
              onChange={(e) => setDateSeance(e.target.value)}
            />
          </label>
          <label className={`block ${dash.textMid}`}>
            Classe
            <select
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={classe}
              onChange={(e) => setClasse(e.target.value)}
            >
              {classes.length === 0 ? <option value="">—</option> : null}
              {classes.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </label>
          {!classes.includes(classe) && classe ? (
            <p className="text-xs text-amber-700">Classe saisie manuellement : {classe}</p>
          ) : null}
          <label className={`block ${dash.textMid}`}>
            Ou classe libre
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={classe}
              onChange={(e) => setClasse(e.target.value)}
              placeholder="ex. 4B"
            />
          </label>
          <label className={`block ${dash.textMid}`}>
            Matière
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={matiereLibelle}
              onChange={(e) => setMatiereLibelle(e.target.value)}
              placeholder="ex. Mathématiques"
            />
          </label>
          <label className={`block ${dash.textMid}`}>
            Contenu de la leçon
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 min-h-[90px]"
              value={contenu}
              onChange={(e) => setContenu(e.target.value)}
              placeholder="Ce qui a été fait en classe…"
            />
          </label>
          <label className={`block ${dash.textMid}`}>
            Travail à faire
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 min-h-[70px]"
              value={travail}
              onChange={(e) => setTravail(e.target.value)}
              placeholder="Exercices, lecture…"
            />
          </label>
          <label className={`block ${dash.textMid}`}>
            À rendre le (optionnel)
            <input
              type="date"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={aRendreLe}
              onChange={(e) => setARendreLe(e.target.value)}
            />
          </label>
          <button
            type="button"
            disabled={busy || !classe.trim() || (!contenu.trim() && !travail.trim())}
            onClick={() => void create()}
            className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-40"
          >
            Enregistrer
          </button>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-4 text-sm">
          <div className="flex flex-wrap items-center justify-between gap-2 mb-3">
            <h2 className="font-bold text-slate-900">Séances</h2>
            <select
              className="rounded-lg border border-slate-200 px-2 py-1 text-xs"
              value={filterClasse}
              onChange={(e) => setFilterClasse(e.target.value)}
            >
              <option value="">Toutes les classes</option>
              {classes.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </div>
          {busy && !entries.length ? (
            <p className={dash.textMid}>Chargement…</p>
          ) : entries.length === 0 ? (
            <p className={dash.textMid}>Aucune séance pour le moment.</p>
          ) : (
            <ul className="space-y-3 max-h-[520px] overflow-auto">
              {entries.map((e) => (
                <li key={e.id} className="rounded-xl border border-slate-200 p-3">
                  <div className="flex flex-wrap items-baseline justify-between gap-2">
                    <div className="font-bold text-slate-900">
                      {e.dateSeance} · {e.classe || "?"}
                      {e.matiereLibelle ? ` · ${e.matiereLibelle}` : ""}
                    </div>
                    <span
                      className={`text-[10px] font-bold uppercase ${
                        e.visibleFamille ? "text-emerald-700" : "text-slate-400"
                      }`}
                    >
                      {e.visibleFamille ? "Familles" : "Masqué"}
                    </span>
                  </div>
                  {e.contenu ? (
                    <p className="mt-1 text-slate-700 whitespace-pre-wrap">{e.contenu}</p>
                  ) : null}
                  {e.travail ? (
                    <p className="mt-1 text-indigo-800">
                      <span className="font-bold">À faire :</span> {e.travail}
                      {e.aRendreLe ? ` (rendre le ${e.aRendreLe})` : ""}
                    </p>
                  ) : null}
                  <div className={`mt-2 text-xs ${dash.textMid}`}>
                    {e.enseignantNom || "Enseignant"}
                    {e.visibleFamille ? (
                      <button
                        type="button"
                        className="ml-3 font-bold text-rose-700 hover:underline"
                        disabled={busy}
                        onClick={() => void hide(e.id)}
                      >
                        Masquer aux familles
                      </button>
                    ) : null}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </ModulePageShell>
  );
}
