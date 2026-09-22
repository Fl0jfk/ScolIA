import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import SanteInaptitudesClient from "@/app/components/sante/SanteInaptitudesClient";

export default function SanteInaptitudesPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Santé"
        title="Inaptitudes EPS"
        description="Certificat daté → extrait EPS. Soft-désactivation — pas de suppression."
        actions={
          <Link href="/sante/espace" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Espace Santé
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <SanteInaptitudesClient />
      </Suspense>
    </ModulePageShell>
  );
}
