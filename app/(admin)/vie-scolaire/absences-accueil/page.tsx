import { redirect } from "next/navigation";

/** Ancienne URL — module Absences unifié (`?tab=consulter`). */
export default function AbsencesAccueilConsultationPage() {
  redirect("/vie-scolaire/absences?tab=consulter");
}
