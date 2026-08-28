"use client";

import { useCallback, useEffect, useState } from "react";
import FamillePortailChrome from "@/app/components/famille/FamillePortailChrome";
import { formatEncoursMontant } from "@/app/lib/foyer-display";

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

type EncoursRow = {
  anneeLabel: string | null;
  montantRestant: number;
  factureCount: number;
};

type Block = {
  foyer: { id: string; label: string; ville: string | null; payeurEstFoyer?: boolean };
  facturation: {
    acceptePrelevement: boolean;
    iban: string | null;
    rum: string | null;
    categorieQuotient: string | null;
  } | null;
  encoursParAnnee?: EncoursRow[];
  factures: FactureRow[];
};

type Item = {
  eleve: { id: string; nom: string; prenom: string; classe: string | null };
  finances: Block[];
};

const STATUT_LABEL: Record<string, string> = {
  emise: "Émise",
  soldee: "Payée",
  partiellement_payee: "Partiellement payée",
  annulee: "Annulée",
};

function FinancesContent({ eleveId }: { eleveId: string | null }) {
  const [items, setItems] = useState<Item[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(true);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const qs = eleveId ? `?eleveId=${encodeURIComponent(eleveId)}` : "";
      const res = await fetch(`/api/famille/finances${qs}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Chargement impossible");
      setItems(data.items || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }, [eleveId]);

  useEffect(() => {
    void load();
  }, [load]);

  if (error) {
    return (
      <div className="rounded-2xl border border-red-200 bg-red-50 p-4 text-red-800 text-sm">
        {error}
      </div>
    );
  }
  if (busy) return <p className="text-sm text-slate-500">Chargement…</p>;

  if (!items.length) {
    return <p className="text-sm text-slate-500">Aucune information financière pour le moment.</p>;
  }

  return (
    <>
      {items.map((item) => (
        <section key={item.eleve.id} className="space-y-3">
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
                  <p className="text-xs font-bold uppercase tracking-wide text-indigo-600">
                    Facturation foyer
                  </p>
                  <p className="font-semibold text-slate-900">{block.foyer.label}</p>
                  <p className="text-xs text-slate-600 mt-1">
                    {block.foyer.payeurEstFoyer !== false
                      ? "Compte facturation partagé — chaque responsable avec autorité parentale peut agir avec son propre compte."
                      : "Responsable payeur désigné sur le dossier."}
                  </p>
                  {block.facturation?.acceptePrelevement ? (
                    <p className="text-xs text-emerald-800 mt-1">
                      Prélèvement SEPA actif
                      {block.facturation.iban ? ` · IBAN ${block.facturation.iban}` : ""}
                    </p>
                  ) : (
                    <p className="text-xs text-slate-500 mt-1">Pas de prélèvement SEPA enregistré</p>
                  )}
                  {!block.facturation?.categorieQuotient ? (
                    <p className="text-xs text-amber-800 mt-2 rounded-lg bg-amber-50 px-2 py-1">
                      Catégorie tarifaire à renseigner pour la rentrée (quotient en attente).
                    </p>
                  ) : null}
                </div>

                {(block.encoursParAnnee ?? []).some((e) => e.montantRestant > 0) ? (
                  <div className="rounded-xl border border-amber-200 bg-amber-50/80 px-3 py-2 text-xs">
                    <p className="font-bold text-amber-950">Reste à payer</p>
                    <ul className="mt-1 space-y-0.5 text-amber-900">
                      {block.encoursParAnnee!
                        .filter((e) => e.montantRestant > 0)
                        .map((e) => (
                          <li key={e.anneeLabel ?? "na"}>
                            {e.anneeLabel ?? "Hors année"} : {formatEncoursMontant(e.montantRestant)}
                            {e.factureCount > 1 ? ` (${e.factureCount} factures)` : ""}
                          </li>
                        ))}
                    </ul>
                  </div>
                ) : null}

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
    </>
  );
}

export default function FamilleFinancesClient() {
  return (
    <FamillePortailChrome
      title="Finances"
      description="Factures émises et prélèvement SEPA au nom du foyer (lecture seule)."
    >
      {({ selectedEnfantId }) => <FinancesContent eleveId={selectedEnfantId} />}
    </FamillePortailChrome>
  );
}
