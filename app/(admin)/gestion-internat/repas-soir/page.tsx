import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import InternatRepasSoirClient from "@/app/components/internat/InternatRepasSoirClient";

export default function InternatRepasSoirPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Internat"
        title="Repas du soir"
        description="Droit soir des internes logés contre passage self (après 16 h)."
        actions={
          <div className="flex flex-wrap gap-3 text-sm font-bold">
            <Link href="/gestion-internat/appel-soir" className="text-indigo-600 hover:underline">
              Appel du soir
            </Link>
            <Link href="/gestion-internat" className="text-indigo-600 hover:underline">
              ← Hub JSON
            </Link>
          </div>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <InternatRepasSoirClient />
      </Suspense>
    </ModulePageShell>
  );
}
