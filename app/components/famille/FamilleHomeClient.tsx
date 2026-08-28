"use client";

import Link from "next/link";
import FamillePortailChrome from "@/app/components/famille/FamillePortailChrome";

export default function FamilleHomeClient() {
  return (
    <FamillePortailChrome
      title="Bienvenue"
      description="Bulletins, absences, carnet et factures de vos enfants."
    >
      {({ selectedEnfant, enfants }) => (
        <>
          <section className="rounded-2xl border border-slate-200 bg-white p-4">
            <h2 className="font-bold text-slate-900">Vos enfants</h2>
            {enfants.length === 0 ? (
              <p className="text-sm text-slate-600 mt-2">Aucun enfant rattaché pour le moment.</p>
            ) : (
              <ul className="mt-3 space-y-2">
                {enfants.map((e) => (
                  <li key={e.id} className="rounded-xl border border-slate-100 bg-slate-50 px-3 py-2">
                    <div className="flex justify-between gap-2 text-sm">
                      <span className="font-semibold">
                        {e.prenom} {e.nom}
                      </span>
                      <span className="text-slate-500">{e.classe || "—"}</span>
                    </div>
                    {e.foyers.length > 0 ? (
                      <p className="text-xs text-indigo-800 mt-1">
                        Foyer : {e.foyers.map((f) => f.label).join(", ")}
                      </p>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </section>

          {selectedEnfant ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <Link
                href={`/famille/bulletins?enfant=${encodeURIComponent(selectedEnfant.id)}`}
                className="rounded-2xl border border-indigo-200 bg-indigo-50 p-4 hover:bg-indigo-100"
              >
                <p className="font-bold text-indigo-900">Bulletins</p>
                <p className="text-xs text-indigo-800 mt-1">Moyennes et PDF publiés</p>
              </Link>
              <Link
                href={`/famille/absences?enfant=${encodeURIComponent(selectedEnfant.id)}`}
                className="rounded-2xl border border-indigo-200 bg-indigo-50 p-4 hover:bg-indigo-100"
              >
                <p className="font-bold text-indigo-900">Absences</p>
                <p className="text-xs text-indigo-800 mt-1">Absences et retards</p>
              </Link>
              <Link
                href={`/famille/carnet?enfant=${encodeURIComponent(selectedEnfant.id)}`}
                className="rounded-2xl border border-indigo-200 bg-indigo-50 p-4 hover:bg-indigo-100"
              >
                <p className="font-bold text-indigo-900">Carnet</p>
                <p className="text-xs text-indigo-800 mt-1">Messages avec accusé de lecture</p>
              </Link>
              <Link
                href={`/famille/finances?enfant=${encodeURIComponent(selectedEnfant.id)}`}
                className="rounded-2xl border border-indigo-200 bg-indigo-50 p-4 hover:bg-indigo-100"
              >
                <p className="font-bold text-indigo-900">Finances</p>
                <p className="text-xs text-indigo-800 mt-1">Factures et prélèvement SEPA</p>
              </Link>
            </div>
          ) : null}
        </>
      )}
    </FamillePortailChrome>
  );
}
