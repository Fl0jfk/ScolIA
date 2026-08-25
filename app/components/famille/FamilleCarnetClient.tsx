"use client";

import { useCallback, useEffect, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";

type Entree = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateEntree: string;
  categorie: string;
  titre: string;
  corps: string;
  signeAt: string | null;
  signeParNom: string | null;
  createdByNom: string | null;
};

const CAT_LABEL: Record<string, string> = {
  correspondance: "Correspondance",
  accompagnement: "Accompagnement",
  information: "Information",
};

export default function FamilleCarnetClient() {
  const [entrees, setEntrees] = useState<Entree[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/famille/carnet", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setEntrees(data.entrees || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const signer = async (id: string) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch("/api/famille/carnet", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "signer", id }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Signature impossible");
      setMessage("Accusé enregistré.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-b from-indigo-50 to-white">
      <header className="border-b border-indigo-100 bg-white/90 backdrop-blur px-4 py-4">
        <div className="max-w-2xl mx-auto">
          <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">Espace famille</p>
          <h1 className="text-2xl font-black text-slate-900 mt-0.5">Carnet</h1>
          <p className="text-sm text-slate-600 mt-1">
            Messages de l&apos;établissement — accusez lecture pour confirmer.
          </p>
          <FamilleNav />
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-6 space-y-4">
        {error && (
          <div className="rounded-2xl border border-red-200 bg-red-50 p-4 text-red-800 text-sm">{error}</div>
        )}
        {message && (
          <div className="rounded-2xl border border-emerald-200 bg-emerald-50 p-4 text-emerald-800 text-sm">
            {message}
          </div>
        )}

        {entrees.length === 0 && !error ? (
          <p className="text-sm text-slate-500">Aucune entrée dans le carnet pour le moment.</p>
        ) : (
          <ul className="space-y-3">
            {entrees.map((e) => (
              <li key={e.id} className="rounded-2xl border border-slate-200 bg-white p-4">
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <div>
                    <p className="text-xs text-slate-500">
                      {e.elevePrenom} {e.eleveNom}
                      {e.eleveClasse ? ` · ${e.eleveClasse}` : ""} ·{" "}
                      {e.dateEntree ? new Date(e.dateEntree).toLocaleDateString("fr-FR") : "—"} ·{" "}
                      {CAT_LABEL[e.categorie] || e.categorie}
                    </p>
                    <p className="font-bold text-slate-900 mt-1">{e.titre}</p>
                  </div>
                  <span
                    className={`rounded-full px-2.5 py-0.5 text-[11px] font-bold ${
                      e.signeAt ? "bg-emerald-50 text-emerald-800" : "bg-amber-50 text-amber-900"
                    }`}
                  >
                    {e.signeAt ? "Signé" : "À signer"}
                  </span>
                </div>
                <p className="mt-2 text-sm text-slate-700 whitespace-pre-wrap">{e.corps}</p>
                {e.createdByNom ? (
                  <p className="mt-2 text-xs text-slate-400">De {e.createdByNom}</p>
                ) : null}
                {!e.signeAt ? (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void signer(e.id)}
                    className="mt-3 bg-indigo-600 text-white px-4 py-2 rounded-xl text-sm font-bold disabled:opacity-40"
                  >
                    Accuser lecture
                  </button>
                ) : (
                  <p className="mt-2 text-xs text-emerald-700">
                    Accusé le {new Date(e.signeAt).toLocaleString("fr-FR")}
                  </p>
                )}
              </li>
            ))}
          </ul>
        )}
      </main>
    </div>
  );
}
