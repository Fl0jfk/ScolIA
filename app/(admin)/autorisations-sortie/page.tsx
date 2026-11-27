import { redirect } from "next/navigation";

/** Ancienne adresse (liens déjà envoyés par e-mail) : la page est désormais un onglet de l'Internat. */
export default function AutorisationsSortiePage() {
  redirect("/gestion-internat?tab=sorties");
}
