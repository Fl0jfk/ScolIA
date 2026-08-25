"use client";

import { useCallback, useEffect, useState } from "react";
import FamilleNav from "@/app/components/famille/FamilleNav";

type FactureRow = {
  id: string;
  numero: string;
  statut: string;
  totalTtc: string;
  dateEmission: string | null;
  dateEcheance: string | null;
  enRetard: boolean;
  hasPdf: boolean;
};

type Block = {
  foyer: { id: string; label: string; ville: string | null };
  facturation: {
    acceptePrelevement: boolean;
    iban: string | null;
    rum: string | null;
  } | null;
  factures: FactureRow[];
};

type Item = {
  eleve: { id: string; nom: string; prenom: string; classe: string | null };
  finances: Block[];
};

const STATUT_LABEL: Record<string, string> = {
  emise: "Émise",
  payee: "Payée",
  partielle: "Partiellement payée",
  annulee: "Annulée",
};

export default function FamilleFinancesClient() {
  const [items, setItems] = useState<Item[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(true);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/famille/finances", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setItems(data.items || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
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
          <h1 className="text-2xl font-black text-slate-900 mt-0.5">Finances</h1>
          <p className="text-sm text-slate-600 mt-1">
            Factures émises et prélèvement SEPA (lecture seule).
          </p>
          <FamilleNav />
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-6 space-y-4">
        {error && (
          <div className="rounded-2xl border border-red-200 bg-red-50 p-4 text-red-800 text-sm">
            {error}
          </div>
        )}
        {busy && <p className="text-sm text-slate-500">Chargement…</p>}

        {!busy &&
          items.map((item) => (
            <section key={item.eleve.id} className="space-y-3">
              <h2 className="font-bold text-slate-900">
                {item.eleve.prenom} {item.eleve.nom}
                {item.eleve.classe ? (
                  <span className="text-slate-500 font-medium"> · {item.eleve.classe}</span>
                ) : null}
              </h2>

              {item.finances.length === 0 ? (
                <p className="text-sm text-slate-500 rounded-2xl border border-slate-200 bg-white p-4">
                  Aucun foyer facturable rattaché.
                </p>
              ) : (
                item.finances.map((block) => (
                  <div
                    key={block.foyer.id}
                    className="rounded-2xl border border-slate-200 bg-white p-4 space-y-3"
                  >
                    <div>
                      <p className="font-semibold text-slate-900">{block.foyer.label}</p>
                      {block.facturation?.acceptePrelevement ? (
                        <p className="text-xs text-emerald-800 mt-1">
                          Prélèvement SEPA actif
                          {block.facturation.iban ? ` · IBAN ${block.facturation.iban}` : ""}
                        </p>
                      ) : (
                        <p className="text-xs text-slate-500 mt-1">Pas de prélèvement SEPA enregistré</p>
                      )}
                    </div>

                    {block.factures.length === 0 ? (
                      <p className="text-sm text-slate-500">Aucune facture émise.</p>
                    ) : (
                      <ul className="space-y-2">
                        {block.factures.map((f) => (
                          <li
                            key={f.id}
                            className={`rounded-xl border px-3 py-2 text-sm flex flex-wrap items-center justify-between gap-2 ${
                              f.enRetard
                                ? "border-amber-200 bg-amber-50"
                                : "border-slate-100 bg-slate-50"
                            }`}
                          >
                            <div>
                              <p className="font-bold text-slate-900">{f.numero}</p>
                              <p className="text-xs text-slate-600">
                                {STATUT_LABEL[f.statut] || f.statut}
                                {f.enRetard ? " · en retard" : ""}
                                {f.dateEmission ? ` · émise ${f.dateEmission}` : ""}
                              </p>
                            </div>
                            <div className="flex items-center gap-3">
                              <span className="font-semibold">{f.totalTtc} €</span>
                              {f.hasPdf ? (
                                <a
                                  href={`/api/famille/finances/pdf?factureId=${f.id}`}
                                  target="_blank"
                                  rel="noreferrer"
                                  className="text-xs font-bold text-indigo-700 hover:underline"
                                >
                                  PDF
                                </a>
                              ) : null}
                            </div>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                ))
              )}
            </section>
          ))}

        {!busy && !error && items.length === 0 ? (
          <p className="text-sm text-slate-500">Aucune information financière pour le moment.</p>
        ) : null}
      </main>
    </div>
  );
}
