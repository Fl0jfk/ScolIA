import Link from "next/link";
import { Suspense } from "react";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import StagesPresenceClient from "@/app/components/stages/StagesPresenceClient";

export default function StagesPresencePage() {
  return (
    <ModulePageShell maxWidthClass="max-w-3xl">
      <ModulePageHeader
        eyebrow="Stages"
        title="Présence — élèves en stage"
        description="Lien présence / bulletin : en stage = ailleurs, pas une absence bulletin."
        actions={
          <Link href="/stages" className="text-sm font-bold text-indigo-600 hover:underline">
            ← Stages & conventions
          </Link>
        }
      />
      <Suspense fallback={<p className="text-sm text-slate-500">Chargement…</p>}>
        <StagesPresenceClient />
      </Suspense>
    </ModulePageShell>
  );
}
