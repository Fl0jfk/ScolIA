import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import PassagesClient from "@/app/components/passages/PassagesClient";

export default function PassagesPortailPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Passages"
        title="Portail"
        description="Entrée / sortie. Occupancy lit la dernière sortie du jour."
        actions={
          <Link href="/passages" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Passages & cantine
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <PassagesClient lieu="portail" />
      </Suspense>
    </ModulePageShell>
  );
}
