import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import SantePaiClient from "@/app/components/sante/SantePaiClient";

export default function SantePaiPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="PAI"
        description="Protocole, traitements autorisés, document. L’infirmerie valide. Soft-révocation — pas de suppression."
        actions={
          <Link href="/sante/espace" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Espace Santé
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <SantePaiClient />
      </Suspense>
    </ModulePageShell>
  );
}
