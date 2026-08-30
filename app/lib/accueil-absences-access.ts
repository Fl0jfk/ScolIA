import { canViewOgecAbsences } from "@/app/lib/absences-types";
import type { AccueilBoardKind } from "@/app/lib/accueil-absences-types";

/**
 * Saisie : tout titulaire du module « Absence accueil » (droits modules).
 * Les CPE / direction l’ont par défaut pour consulter ; l’accueil l’active
 * sur les personnes du standard.
 */
export function canDeclareAccueilAbsence(_roles: string[]): boolean {
  return true;
}

/** Lignes personnel OGEC : uniquement administratif, compta RH, direction. */
export function canSeeAccueilBoardKind(kind: AccueilBoardKind, roles: string[]): boolean {
  if (kind === "ogec") return canViewOgecAbsences(roles);
  return true;
}
