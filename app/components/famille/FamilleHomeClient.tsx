"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";

type Enfant = { id: string; nom: string; prenom: string; classe: string | null };

export default function FamilleHomeClient() {
  const [enfants, setEnfants] = useState<Enfant[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const res = await fetch("/api/famille/enfants", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setEnfants(data.enfants || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
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
          <h1 className="text-2xl font-black text-slate-900 mt-0.5">Bienvenue</h1>
          <p className="text-sm text-slate-600 mt-1">
            Bulletins, absences, carnet et factures de vos enfants.
          </p>
          <FamilleNav />
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-6 space-y-4">
        {error && (
          <div className="rounded-2xl border border-red-200 bg-red-50 p-4 text-red-800 text-sm">
            {error}
            <p className="mt-2">
              <a href="/auth/sign-in" className="font-bold underline">
                Se connecter
              </a>
            </p>
          </div>
        )}

        {!error && (
          <>
            <section className="rounded-2xl border border-slate-200 bg-white p-4">
              <h2 className="font-bold text-slate-900">Vos enfants</h2>
              {enfants.length === 0 ? (
                <p className="text-sm text-slate-600 mt-2">Aucun enfant rattaché pour le moment.</p>
              ) : (
                <ul className="mt-3 space-y-2">
                  {enfants.map((e) => (
                    <li key={e.id} className="text-sm flex justify-between gap-2">
                      <span className="font-semibold">
                        {e.prenom} {e.nom}
                      </span>
                      <span className="text-slate-500">{e.classe || "—"}</span>
                    </li>
                  ))}
                </ul>
              )}
            </section>

            <div className="grid gap-3 sm:grid-cols-2">
              <Link
                href="/famille/bulletins"
                className="rounded-2xl border border-indigo-200 bg-indigo-50 p-4 hover:bg-indigo-100"
              >
                <p className="font-bold text-indigo-900">Bulletins</p>
                <p className="text-xs text-indigo-800 mt-1">Moyennes et PDF publiés</p>
              </Link>
              <Link
                href="/famille/absences"
                className="rounded-2xl border border-slate-200 bg-white p-4 hover:bg-slate-50"
              >
                <p className="font-bold text-slate-900">Absences</p>
                <p className="text-xs text-slate-600 mt-1">Absences et retards signalés</p>
              </Link>
              <Link
                href="/famille/carnet"
                className="rounded-2xl border border-slate-200 bg-white p-4 hover:bg-slate-50"
              >
                <p className="font-bold text-slate-900">Carnet</p>
                <p className="text-xs text-slate-600 mt-1">Messages — accusé de lecture</p>
              </Link>
              <Link
                href="/famille/finances"
                className="rounded-2xl border border-slate-200 bg-white p-4 hover:bg-slate-50"
              >
                <p className="font-bold text-slate-900">Finances</p>
                <p className="text-xs text-slate-600 mt-1">Factures et prélèvement SEPA</p>
              </Link>
            </div>
          </>
        )}
      </main>
    </div>
  );
}
