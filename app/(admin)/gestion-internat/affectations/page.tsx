import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import InternatAffectationsClient from "@/app/components/internat/InternatAffectationsClient";

export default function InternatAffectationsPage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Internat"
        title="Chambres & lits datés"
        description="Affectation Postgres (date début / fin). Le stockage JSON existant n’est pas modifié."
        actions={
          <Link
            href="/gestion-internat"
            className="text-sm font-bold text-indigo-600 hover:underline"
          >
            ← Hub internat (JSON)
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <InternatAffectationsClient />
      </Suspense>
    </ModulePageShell>
  );
}
