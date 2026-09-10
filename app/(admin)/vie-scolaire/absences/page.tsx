import { Suspense } from "react";
import VsAbsencesHubClient from "@/app/components/vie-scolaire/VsAbsencesHubClient";

/** Module Absences unifié : onglets selon les droits (déclarer / consulter / appels). */
export default function VsAbsencesHubPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement…</p>}>
      <VsAbsencesHubClient />
    </Suspense>
  );
}
