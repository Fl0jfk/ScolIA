import { redirect } from "next/navigation";

/** Ancienne URL — module Absences unifié (`?tab=declarer`). */
export default function AccueilAbsencesPage() {
  redirect("/vie-scolaire/absences?tab=declarer");
}
