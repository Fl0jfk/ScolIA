/** Corps de notification : le `detail` du tableau de bord peut citer une personne ou un dossier. */

/** Signaux dont le `detail` peut contenir un nom (personne, famille, dossier) -> texte générique. */
const GENERIC_BODY_PREFIXES: Array<[string, (label: string) => string]> = [
  ["absences-pending", () => "Nouvelle activité dans Absences"],
  ["absences-admin-queue", () => "Nouvelle activité dans Absences"],
  ["requests-claimed", () => "Nouvelle activité dans Demandes"],
  ["requests-pool", () => "Nouvelle activité dans Demandes"],
  ["photocopies-ops-queue", () => "Nouvelle activité dans Photocopies"],
  ["photo-dir", () => "Nouvelle activité dans Photocopies"],
  ["documents-share-", () => "Un dossier vous a été partagé"],
  ["eleve-accompagnement-", () => "Nouvelle alerte d'accompagnement"],
];

export function notificationBody(n: { id: string; label: string; detail: string }): string {
  for (const [prefix, make] of GENERIC_BODY_PREFIXES) {
    if (n.id === prefix || n.id.startsWith(prefix)) return make(n.label);
  }
  return n.detail?.trim() || "Nouveauté dans ScolIA";
}

/** Titre : générique quand le label lui-même est sensible (type d'alerte d'accompagnement en majuscules). */
export function notificationTitle(n: { id: string; label: string }): string {
  return n.id.startsWith("eleve-accompagnement-") ? "Accompagnement" : n.label;
}
