import { Suspense } from "react";
import ElevesHubClient from "@/app/(admin)/eleves/ElevesHubClient";

export default function ElevesHubPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement du module Élèves…</p>}>
      <ElevesHubClient />
    </Suspense>
  );
}
