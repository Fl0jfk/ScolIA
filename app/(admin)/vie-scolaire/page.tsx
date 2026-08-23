import { Suspense } from "react";
import PillarHubClient from "@/app/components/module-hub/PillarHubClient";

export default function VieScolaireHubPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement…</p>}>
      <PillarHubClient pillarId="vie_scolaire" loadingLabel="Chargement de la vie scolaire…" />
    </Suspense>
  );
}
