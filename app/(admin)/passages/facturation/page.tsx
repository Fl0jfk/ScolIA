import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import FacturationCantineClient from "@/app/components/passages/FacturationCantineClient";

export default function PassagesFacturationPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Passages"
        title="Facturation cantine"
        description="Réglage forfait ou réel, puis lignes de facture liées au droit ou aux passages self."
        actions={
          <Link href="/passages" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Passages & cantine
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <FacturationCantineClient />
      </Suspense>
    </ModulePageShell>
  );
}
