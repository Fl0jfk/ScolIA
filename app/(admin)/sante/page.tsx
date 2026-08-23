import { Suspense } from "react";
import PillarHubClient from "@/app/components/module-hub/PillarHubClient";

export default function SanteHubPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement…</p>}>
      <PillarHubClient pillarId="sante" loadingLabel="Chargement de l’espace santé…" />
    </Suspense>
  );
}
