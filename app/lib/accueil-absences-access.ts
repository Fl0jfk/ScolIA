import { canViewOgecAbsences } from "@/app/lib/absences-types";
import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";
import type { AccueilBoardKind } from "@/app/lib/accueil-absences-types";

/**
 * Saisie : tout titulaire du module « Absence accueil » (droits modules).
 * Les CPE / direction l’ont par défaut pour consulter ; l’accueil l’active
 * sur les personnes du standard.
 */
export function canDeclareAccueilAbsence(_roles: string[]): boolean {
  return true;
}

/** Lignes personnel OGEC : administratif, compta RH, direction, admin établissement. */
export function canSeeAccueilBoardKind(kind: AccueilBoardKind, roles: string[]): boolean {
  if (kind === "ogec") {
    return canViewOgecAbsences(roles) || hasGlobalAdminRole(roles) || hasMasterRole(roles);
  }
  return true;
}
