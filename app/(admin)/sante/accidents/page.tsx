import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import SanteAccidentsClient from "@/app/components/sante/SanteAccidentsClient";

export default function SanteAccidentsPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Registre des accidents"
        description="Circonstances, soins, suite. Conservation longue — pas de suppression."
        actions={
          <Link href="/sante/espace" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Espace Santé
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <SanteAccidentsClient />
      </Suspense>
    </ModulePageShell>
  );
}
