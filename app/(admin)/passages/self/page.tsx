import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import PassagesClient from "@/app/components/passages/PassagesClient";

export default function PassagesSelfPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Passages"
        title="Self — repas pris"
        description="Le repas pris est un passage. Les extraits cantine s’affichent à la saisie."
        actions={
          <Link href="/passages" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Passages & cantine
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <PassagesClient lieu="self" />
      </Suspense>
    </ModulePageShell>
  );
}
