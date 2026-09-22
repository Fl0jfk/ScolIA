import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import PrevisionRepasClient from "@/app/components/passages/PrevisionRepasClient";

export default function PassagesPrevisionPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Passages"
        title="Prévision des repas"
        description="Droit (grille / régime) contre pris (passage self) pour le jour."
        actions={
          <Link href="/passages" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Passages & cantine
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <PrevisionRepasClient />
      </Suspense>
    </ModulePageShell>
  );
}
