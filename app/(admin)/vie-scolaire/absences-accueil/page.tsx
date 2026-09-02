import { Suspense } from "react";
import AccueilAbsencesConsultationClient from "@/app/components/vie-scolaire/AccueilAbsencesConsultationClient";

export default function AbsencesAccueilConsultationPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement…</p>}>
      <AccueilAbsencesConsultationClient />
    </Suspense>
  );
}
