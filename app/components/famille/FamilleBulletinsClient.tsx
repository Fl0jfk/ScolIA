"use client";

import { useCallback, useEffect, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";

type Enfant = {
  id: string;
  nom: string;
  prenom: string;
  classe: string | null;
  ine: string | null;
};

type Bulletin = {
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  periodeId: string;
  periodeCode: string;
  periodeLibelle: string;
  anneeLabel: string;
  moyenneGenerale: string | null;
  nbMatieres: number;
  nbCompetences: number;
  pdfUrl: string;
};

export default function FamilleBulletinsClient() {
  const [enfants, setEnfants] = useState<Enfant[]>([]);
  const [bulletins, setBulletins] = useState<Bulletin[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/famille/bulletins", { cache: "no-store" });
      const data = (await res.json()) as {
        error?: string;
        enfants?: Enfant[];
        bulletins?: Bulletin[];
      };
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setEnfants(data.enfants || []);
      setBulletins(data.bulletins || []);
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
          <h1 className="text-2xl font-black text-slate-900 mt-0.5">Bulletins scolaires</h1>
          <p className="text-sm text-slate-600 mt-1">
            Bulletins publiés après clôture des périodes par l&apos;établissement.
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
          <section className="rounded-2xl border border-slate-200 bg-white p-4">
            <h2 className="font-bold text-slate-900">Vos enfants</h2>
            <ul className="mt-3 space-y-2">
              {enfants.map((e) => (
                <li
                  key={e.id}
                  className="text-sm flex justify-between gap-2 border-b border-slate-100 pb-2"
                >
                  <span className="font-semibold">
                    {e.prenom} {e.nom}
                  </span>
                  <span className="text-slate-500">{e.classe || "—"}</span>
                </li>
              ))}
            </ul>
          </section>
        )}

        {!loading && !error && bulletins.length === 0 && (
          <p className="text-sm text-slate-600 rounded-2xl border border-slate-200 bg-white p-4">
            Aucun bulletin publié pour le moment. Les bulletins apparaissent dès qu&apos;une période
            est clôturée par l&apos;établissement.
          </p>
        )}

        {!loading && bulletins.length > 0 && (
          <section className="space-y-3">
            <h2 className="font-bold text-slate-900">Bulletins disponibles</h2>
            {bulletins.map((b) => (
              <article
                key={`${b.eleveId}-${b.periodeId}`}
                className="rounded-2xl border border-slate-200 bg-white p-4 flex flex-wrap items-center justify-between gap-3"
              >
                <div>
                  <p className="font-bold text-slate-900">
                    {b.elevePrenom} {b.eleveNom}
                  </p>
                  <p className="text-sm text-slate-600">
                    {b.periodeLibelle} · {b.anneeLabel}
                    {b.eleveClasse ? ` · ${b.eleveClasse}` : ""}
                  </p>
                  <p className="text-xs text-slate-500 mt-1">
                    {b.moyenneGenerale != null ? `Moyenne ${b.moyenneGenerale}` : "Sans moyenne"}
                    {b.nbCompetences > 0 ? ` · ${b.nbCompetences} compétence(s) LSU` : ""}
                  </p>
                </div>
                <a
                  href={b.pdfUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center rounded-xl bg-indigo-600 text-white px-4 py-2 text-sm font-bold hover:bg-indigo-700"
                >
                  Ouvrir PDF
                </a>
              </article>
            ))}
          </section>
        )}
      </main>
    </div>
  );
}
