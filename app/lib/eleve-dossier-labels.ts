/** Libellés affichage dossier élève — safe client + serveur (sans server-only). */

import { isEleveSortantEtablissement } from "@/app/lib/eleve-actif-shared";

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

/** Statut affiché sur la fiche élève (inclut sortie par date même si status=inscrit en base). */
export function eleveDossierStatutLabel(fields: {
  status?: string | null;
  dateSortie?: string | null;
}): string {
  if (isEleveSortantEtablissement(fields)) {
    const d = String(fields.dateSortie ?? "").trim();
    if (d) {
      const [y, m, day] = d.split("-");
      if (y && m && day) {
        return `Sorti le ${day}/${m}/${y}`;
      }
      return `Sorti le ${d}`;
    }
    return eleveStatusLabel(fields.status);
  }
  return eleveStatusLabel(fields.status);
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
