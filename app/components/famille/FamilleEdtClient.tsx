"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";

type Enfant = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
};

type Creneau = {
  id: string;
  jourSemaine: number;
  heureDebut: string;
  heureFin: string;
  classe: string | null;
  matiereLibelle: string | null;
  enseignantNom: string | null;
  salle: string | null;
  semaine: string;
};

type Emploi = {
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  classe: string | null;
  creneaux: Creneau[];
};

const JOURS = [
  { n: 1, label: "Lundi" },
  { n: 2, label: "Mardi" },
  { n: 3, label: "Mercredi" },
  { n: 4, label: "Jeudi" },
  { n: 5, label: "Vendredi" },
] as const;

function titreCreneau(c: Creneau): string {
  return c.matiereLibelle?.trim() || c.enseignantNom?.trim() || "Cours";
}

export default function FamilleEdtClient() {
  const [enfants, setEnfants] = useState<Enfant[]>([]);
  const [emplois, setEmplois] = useState<Emploi[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams(window.location.search);
      const enfantParam = params.get("enfant") || params.get("eleveId");
      const qs = enfantParam ? `?eleveId=${encodeURIComponent(enfantParam)}` : "";
      const res = await fetch(`/api/famille/edt${qs}`, { cache: "no-store" });
      const data = (await res.json()) as {
        error?: string;
        enfants?: Enfant[];
        emplois?: Emploi[];
      };
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      const list = data.enfants || [];
      const jobs = data.emplois || [];
      setEnfants(list);
      setEmplois(jobs);
      setSelectedId((prev) => {
        if (enfantParam && list.some((e) => e.id === enfantParam)) return enfantParam;
        if (prev && list.some((e) => e.id === prev)) return prev;
        return list[0]?.id ?? null;
      });
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const emploi = useMemo(
    () => emplois.find((e) => e.eleveId === selectedId) || null,
    [emplois, selectedId],
  );

  const byJour = useMemo(() => {
    const map = new Map<number, Creneau[]>();
    for (const j of JOURS) map.set(j.n, []);
    for (const c of emploi?.creneaux || []) {
      if (c.jourSemaine < 1 || c.jourSemaine > 5) continue;
      const arr = map.get(c.jourSemaine) || [];
      arr.push(c);
      map.set(c.jourSemaine, arr);
    }
    return map;
  }, [emploi]);

  return (
    <div className="min-h-screen bg-gradient-to-b from-indigo-50 to-white">
      <header className="border-b border-indigo-100 bg-white/90 backdrop-blur px-4 py-4">
        <div className="max-w-3xl mx-auto">
          <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">Espace famille</p>
          <h1 className="text-2xl font-black text-slate-900 mt-0.5">Emploi du temps</h1>
          <p className="text-sm text-slate-600 mt-1">
            Grille de la classe (coque labo — l&apos;app native prendra le relais).
          </p>
          <FamilleNav enfantId={selectedId} />
        </div>
      </header>

      <main className="max-w-3xl mx-auto px-4 py-6 space-y-6">
        {loading && <p className="text-sm text-slate-600">Chargement…</p>}

        {error && (
          <div className="rounded-2xl border border-red-200 bg-red-50 p-4 text-red-800 text-sm">
            {error}
          </div>
        )}

        {!loading && !error && enfants.length > 1 && (
          <div className="flex flex-wrap gap-2">
            {enfants.map((e) => (
              <button
                key={e.id}
                type="button"
                onClick={() => setSelectedId(e.id)}
                className={`rounded-xl px-3 py-2 text-sm font-bold border ${
                  selectedId === e.id
                    ? "bg-indigo-600 text-white border-indigo-600"
                    : "bg-white text-slate-700 border-slate-200"
                }`}
              >
                {e.prenom} {e.nom}
                {e.classe ? (
                  <span className="ml-2 text-xs opacity-80">{e.classe}</span>
                ) : null}
              </button>
            ))}
          </div>
        )}

        {!loading && !error && emploi && (
          <section>
            <h2 className="text-sm font-bold text-slate-800">
              {emploi.elevePrenom} {emploi.eleveNom}
              {emploi.classe ? (
                <span className="ml-2 text-slate-500 font-medium">classe {emploi.classe}</span>
              ) : null}
            </h2>
            {!emploi.classe ? (
              <p className="mt-2 text-sm text-slate-600">Aucune classe renseignée pour cet enfant.</p>
            ) : emploi.creneaux.length === 0 ? (
              <p className="mt-2 rounded-2xl border border-slate-200 bg-white p-4 text-sm text-slate-600">
                Aucun créneau publié pour la classe {emploi.classe}.
              </p>
            ) : (
              <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
                {JOURS.map((j) => {
                  const slots = byJour.get(j.n) || [];
                  return (
                    <div
                      key={j.n}
                      className="rounded-2xl border border-slate-200 bg-white p-3 shadow-sm min-h-[8rem]"
                    >
                      <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">
                        {j.label}
                      </p>
                      <ul className="mt-2 space-y-2">
                        {slots.length === 0 ? (
                          <li className="text-xs text-slate-400">—</li>
                        ) : (
                          slots.map((c) => (
                            <li
                              key={c.id}
                              className="rounded-xl bg-slate-50 border border-slate-100 px-2 py-1.5"
                            >
                              <p className="text-[11px] font-semibold text-slate-500 tabular-nums">
                                {c.heureDebut}–{c.heureFin}
                              </p>
                              <p className="text-sm font-bold text-slate-900 leading-snug">
                                {titreCreneau(c)}
                              </p>
                              {c.salle || c.enseignantNom ? (
                                <p className="text-[11px] text-slate-500 mt-0.5">
                                  {[c.salle, c.enseignantNom].filter(Boolean).join(" · ")}
                                </p>
                              ) : null}
                            </li>
                          ))
                        )}
                      </ul>
                    </div>
                  );
                })}
              </div>
            )}
          </section>
        )}
      </main>
    </div>
  );
}
