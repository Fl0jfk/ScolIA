import { redirect } from "next/navigation";

/** Ancienne URL — fusionnée dans Appels & absences. */
export default function VsAppelsLegacyRedirect() {
  redirect("/vie-scolaire/presence?tab=appel");
}
