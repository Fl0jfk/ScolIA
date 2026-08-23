import { Suspense } from "react";
import ElevesDossiersListClient from "./ElevesDossiersListClient";

export default function ElevesDossiersPage() {
  return (
    <Suspense
      fallback={
        <div className="p-8 text-sm text-slate-500">Chargement des dossiers…</div>
      }
    >
      <ElevesDossiersListClient />
    </Suspense>
  );
}
