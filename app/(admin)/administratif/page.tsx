import { Suspense } from "react";
import PillarHubClient from "@/app/components/module-hub/PillarHubClient";

export default function AdministratifHubPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement…</p>}>
      <PillarHubClient pillarId="administratif" loadingLabel="Chargement de l’espace administratif…" />
    </Suspense>
  );
}
