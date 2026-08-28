/** Libellés affichage dossier élève — safe client + serveur (sans server-only). */

export function eleveStatusLabel(status: string | null | undefined): string {
  switch (String(status || "").trim().toLowerCase()) {
    case "inscrit":
      return "Scolarisé";
    case "preinscrit":
      return "Préinscription";
    case "ancien":
      return "Ancien";
    case "archive":
      return "Archivé";
    default:
      return status?.trim() || "—";
  }
}

export function scolariteStatutLabel(statut: string | null | undefined): string {
  switch (String(statut || "").trim().toLowerCase()) {
    case "en_cours":
      return "Année en cours";
    case "prevue":
      return "Prévue";
    case "terminee":
      return "Terminée";
    case "annulee":
      return "Annulée";
    default:
      return statut?.trim() || "—";
  }
}
