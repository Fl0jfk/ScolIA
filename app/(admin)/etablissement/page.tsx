import { Suspense } from "react";
import EtablissementHubClient from "@/app/(admin)/etablissement/EtablissementHubClient";

export default function EtablissementHubPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement du module Établissement…</p>}>
      <EtablissementHubClient />
    </Suspense>
  );
}
