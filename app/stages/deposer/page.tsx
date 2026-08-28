import { redirect } from "next/navigation";

/** Le dépôt PDF n'est plus la voie élève par défaut — formulaire en ligne sur /stages/preconvention. */
export default function StageDeposerRedirectPage() {
  redirect("/stages/preconvention");
}
