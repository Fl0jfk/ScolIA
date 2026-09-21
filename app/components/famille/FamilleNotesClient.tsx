"use client";

import { useCallback, useEffect, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";

type Enfant = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
};

type Note = {
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  devoirId: string;
  devoirLibelle: string;
  dateDevoir: string | null;
  coefficient: string;
  matiereCode: string;
  matiereLibelle: string;
  periodeCode: string;
  periodeLibelle: string;
  periodeStatut: string;
  valeur: string | null;
  absent: boolean;
  dispense: boolean;
  appreciation: string | null;
};

function formatNote(n: Note): string {
  if (n.absent) return "Absent";
  if (n.dispense) return "Dispensé";
  if (n.valeur == null || n.valeur === "") return "—";
  return n.valeur;
}

function formatDate(iso: string | null): string {
  if (!iso) return "";
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso);
  if (!m) return iso;
  return `${m[3]}/${m[2]}/${m[1]}`;
}

export default function FamilleNotesClient() {
  const [enfants, setEnfants] = useState<Enfant[]>([]);
  const [notes, setNotes] = useState<Note[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/famille/notes", { cache: "no-store" });
      const data = (await res.json()) as {
        error?: string;
        enfants?: Enfant[];
        notes?: Note[];
      };
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setEnfants(data.enfants || []);
      setNotes(data.notes || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="min-h-screen bg-gradient-to-b from-indigo-50 to-white">
      <header className="border-b border-indigo-100 bg-white/90 backdrop-blur px-4 py-4">
        <div className="max-w-2xl mx-auto">
          <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">Espace famille</p>
          <h1 className="text-2xl font-black text-slate-900 mt-0.5">Notes</h1>
          <p className="text-sm text-slate-600 mt-1">
            Notes saisies par les professeurs (coque labo — l&apos;app native prendra le relais).
          </p>
          <FamilleNav />
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-6 space-y-6">
        {loading && <p className="text-sm text-slate-600">Chargement…</p>}

        {error && (
          <div className="rounded-2xl border border-red-200 bg-red-50 p-4 text-red-800 text-sm">
            {error}
            {error.includes("Non autorisé") || error.includes("AUTH") ? (
              <p className="mt-2">
                <a href="/auth/sign-in" className="font-bold underline">
                  Se connecter
                </a>
              </p>
            ) : null}
          </div>
        )}

        {!loading && !error && enfants.length > 0 && (
          <section>
            <h2 className="text-xs font-bold uppercase tracking-wide text-slate-500 mb-2">
              Vos enfants
            </h2>
            <ul className="flex flex-wrap gap-2">
              {enfants.map((e) => (
                <li
                  key={e.id}
                  className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm font-semibold text-slate-800"
                >
                  {e.prenom} {e.nom}
                  {e.classe ? (
                    <span className="ml-2 text-xs font-medium text-slate-500">{e.classe}</span>
                  ) : null}
                </li>
              ))}
            </ul>
          </section>
        )}

        {!loading && !error && notes.length === 0 && (
          <p className="rounded-2xl border border-slate-200 bg-white p-4 text-sm text-slate-600">
            Aucune note publiée pour le moment.
          </p>
        )}

        <ul className="space-y-3">
          {notes.map((n) => (
            <li
              key={`${n.devoirId}:${n.eleveId}`}
              className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm"
            >
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div>
                  <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">
                    {n.matiereLibelle}
                    <span className="ml-2 font-medium text-slate-400">{n.periodeLibelle}</span>
                  </p>
                  <h3 className="text-base font-bold text-slate-900 mt-0.5">{n.devoirLibelle}</h3>
                  <p className="text-xs text-slate-500 mt-1">
                    {n.elevePrenom} {n.eleveNom}
                    {n.eleveClasse ? ` · ${n.eleveClasse}` : ""}
                    {n.dateDevoir ? ` · ${formatDate(n.dateDevoir)}` : ""}
                    {` · coef. ${n.coefficient}`}
                  </p>
                </div>
                <p className="text-2xl font-black text-slate-900 tabular-nums">{formatNote(n)}</p>
              </div>
              {n.appreciation ? (
                <p className="mt-2 text-sm text-slate-600 border-t border-slate-100 pt-2">
                  {n.appreciation}
                </p>
              ) : null}
            </li>
          ))}
        </ul>
      </main>
    </div>
  );
}
