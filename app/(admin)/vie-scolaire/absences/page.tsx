import { Suspense } from "react";
import VsAbsencesClient from "@/app/components/vie-scolaire/VsAbsencesClient";

export default function VsAbsencesPage() {
  return (
    <Suspense fallback={<p className="p-8 text-center text-slate-500">Chargement des absences…</p>}>
      <VsAbsencesClient />
    </Suspense>
  );
}
