import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import SanteMedicamentsClient from "@/app/components/sante/SanteMedicamentsClient";

export default function SanteMedicamentsPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Médicaments"
        description="Journal des prises à l’infirmerie. L’ordonnance reste un document."
        actions={
          <Link href="/sante/espace" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Espace Santé
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <SanteMedicamentsClient />
      </Suspense>
    </ModulePageShell>
  );
}
