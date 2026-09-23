"use client";

import { useCallback, useEffect, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";

type Entry = {
  id: string;
  dateSeance: string;
  classe: string | null;
  matiereLibelle: string | null;
  contenu: string;
  travail: string;
  aRendreLe: string | null;
  enseignantNom: string | null;
  enfantsConcernes: Array<{ id: string; nom: string; prenom: string; classe: string | null }>;
};

type Enfant = { id: string; nom: string; prenom: string; classe: string | null };

export default function FamilleCahierClient() {
  const [entries, setEntries] = useState<Entry[]>([]);
  const [enfants, setEnfants] = useState<Enfant[]>([]);
  const [enfantId, setEnfantId] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const qs = new URLSearchParams();
      if (enfantId) qs.set("enfant", enfantId);
      const res = await fetch(`/api/famille/cahier-texte?${qs}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setEntries(data.entries || []);
      setEnfants(data.enfants || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, [enfantId]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="min-h-screen bg-slate-50 text-slate-900">
      <div className="mx-auto max-w-3xl px-4 py-8">
        <header className="mb-6">
          <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">Portail familles</p>
          <h1 className="text-2xl font-black">Cahier de textes</h1>
          <p className="text-sm text-slate-600 mt-1">
            Leçons et travail à faire pour la classe de votre enfant.
          </p>
          <FamilleNav enfantId={enfantId || null} />
        </header>

        {error ? <p className="mb-3 text-sm font-semibold text-rose-700">{error}</p> : null}

        {enfants.length > 1 ? (
          <label className="mb-4 block text-sm">
            Enfant
            <select
              className="mt-1 w-full max-w-xs rounded-lg border border-slate-200 px-3 py-2"
              value={enfantId}
              onChange={(e) => setEnfantId(e.target.value)}
            >
              <option value="">Tous</option>
              {enfants.map((e) => (
                <option key={e.id} value={e.id}>
                  {e.prenom} {e.nom}
                  {e.classe ? ` (${e.classe})` : ""}
                </option>
              ))}
            </select>
          </label>
        ) : null}

        {busy && !entries.length ? (
          <p className="text-sm text-slate-500">Chargement…</p>
        ) : entries.length === 0 ? (
          <p className="text-sm text-slate-500">Aucune séance publiée pour le moment.</p>
        ) : (
          <ul className="space-y-3">
            {entries.map((e) => (
              <li key={e.id} className="rounded-2xl border border-slate-200 bg-white p-4 text-sm">
                <div className="font-bold text-slate-900">
                  {e.dateSeance} · {e.classe || "?"}
                  {e.matiereLibelle ? ` · ${e.matiereLibelle}` : ""}
                </div>
                {e.enfantsConcernes?.length ? (
                  <p className="text-xs text-slate-500 mt-0.5">
                    Pour{" "}
                    {e.enfantsConcernes
                      .map((x) => `${x.prenom} ${x.nom}`)
                      .join(", ")}
                  </p>
                ) : null}
                {e.contenu ? (
                  <p className="mt-2 whitespace-pre-wrap text-slate-800">{e.contenu}</p>
                ) : null}
                {e.travail ? (
                  <p className="mt-2 rounded-xl bg-indigo-50 border border-indigo-100 px-3 py-2 text-indigo-900">
                    <span className="font-bold">Travail à faire</span>
                    {e.aRendreLe ? ` — à rendre le ${e.aRendreLe}` : ""}
                    <br />
                    {e.travail}
                  </p>
                ) : null}
                {e.enseignantNom ? (
                  <p className="mt-2 text-xs text-slate-500">{e.enseignantNom}</p>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
