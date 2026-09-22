import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import InfirmerieFicheClient from "@/app/components/sante/InfirmerieFicheClient";

export default function InfirmerieFichesPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Fiches infirmerie"
        description="Antécédents utiles à l’établissement et personnes à prévenir. Pas le dossier médical national."
        actions={
          <Link href="/sante/espace" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Espace Santé
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <InfirmerieFicheClient />
      </Suspense>
    </ModulePageShell>
  );
}
