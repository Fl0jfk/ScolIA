import { Suspense } from "react";
import EleveDossierClient from "./EleveDossierClient";

export default function EleveDossierPage() {
  return (
    <Suspense
      fallback={
        <div className="p-8 text-sm text-slate-500">Chargement du dossier…</div>
      }
    >
      <EleveDossierClient />
    </Suspense>
  );
}
