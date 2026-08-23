import { Suspense } from "react";
import PillarHubClient from "@/app/components/module-hub/PillarHubClient";

export default function ComptaRhHubPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement…</p>}>
      <PillarHubClient pillarId="compta_rh" loadingLabel="Chargement Comptabilité & RH…" />
    </Suspense>
  );
}
