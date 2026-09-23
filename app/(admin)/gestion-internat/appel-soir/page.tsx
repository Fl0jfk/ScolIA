import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import InternatAppelSoirClient from "@/app/components/internat/InternatAppelSoirClient";

export default function InternatAppelSoirPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Internat"
        title="Appel du soir"
        description="Présents / absents sur les lits datés Postgres. Le roll-call JSON n’est pas modifié."
        actions={
          <div className="flex flex-wrap gap-3 text-sm font-bold">
            <Link href="/gestion-internat/affectations" className="text-indigo-600 hover:underline">
              Lits datés
            </Link>
            <Link href="/gestion-internat" className="text-indigo-600 hover:underline">
              ← Hub JSON
            </Link>
          </div>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <InternatAppelSoirClient />
      </Suspense>
    </ModulePageShell>
  );
}
