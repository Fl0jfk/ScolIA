import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import InfirmeriePassagesClient from "@/app/components/sante/InfirmeriePassagesClient";

export default function InfirmeriePassagesPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Passages infirmerie"
        description="Enregistrer qui est à l’infirmerie. La vie scolaire lit le signal, pas le motif médical."
        actions={
          <Link href="/sante/espace" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Espace Santé
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <InfirmeriePassagesClient />
      </Suspense>
    </ModulePageShell>
  );
}
