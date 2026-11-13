"use client";

import Link from "@/app/components/ui/AppLink";
import type { PersonnelIndexEntry } from "@/app/lib/personnel-types";
import RhDriveLinkPanel from "@/app/components/personnel/RhDriveLinkPanel";

export default function RhAdminOverviewPanel({ index }: { index: PersonnelIndexEntry[] }) {
  const onboarding = index.filter(
    (p) => p.onboardingStatus && ["brouillon", "en_cours", "signatures"].includes(p.onboardingStatus),
  );

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-indigo-100 bg-indigo-50/50 p-4 text-sm text-indigo-950">
        Parcours d&apos;entrée et de sortie — signatures par lien e-mail (sans paie ni coffre bulletins).
      </div>

      <RhDriveLinkPanel />

      <section className="bg-white rounded-2xl border border-slate-200 p-5 flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 className="font-black text-slate-900">Registre unique du personnel</h3>
          <p className="text-sm text-slate-600 mt-1">
            Tableau conformité OneDrive, alertes SST / formations / médecine, compétences.
          </p>
        </div>
        <Link
          href="/rh?tab=registre"
          className="px-4 py-2 rounded-xl bg-indigo-600 text-white text-xs font-bold shadow-sm hover:bg-indigo-700"
        >
          Ouvrir le registre
        </Link>
      </section>

      <section className="bg-white rounded-2xl border border-slate-200 p-5">
        <h3 className="font-black text-slate-900 mb-3">Intégrations en cours</h3>
        {onboarding.length === 0 ? (
          <p className="text-sm text-slate-500">Aucune entrée en cours.</p>
        ) : (
          <ul className="space-y-2">
            {onboarding.map((p) => (
              <li key={p.id}>
                <Link
                  href={`/rh/${p.id}`}
                  className="flex justify-between rounded-xl border border-slate-100 px-4 py-3 hover:bg-slate-50"
                >
                  <span className="font-bold text-slate-900">{p.displayName}</span>
                  <span className="text-xs font-bold text-indigo-700 capitalize">
                    {p.onboardingStatus?.replace(/_/g, " ")}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="bg-white rounded-2xl border border-slate-200 p-5">
        <h3 className="font-black text-slate-900 mb-2">Démarrer une sortie</h3>
        <p className="text-sm text-slate-600 mb-3">
          Ouvrez la fiche du collaborateur → onglet <strong>Offboarding</strong> pour lancer le
          parcours de départ.
        </p>
        <p className="text-xs text-slate-500">
          Checklist restitution + signatures direction / compta / employé, comme pour
          l&apos;entrée.
        </p>
      </section>
    </div>
  );
}
