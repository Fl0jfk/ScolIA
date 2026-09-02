import { canViewOgecAbsences } from "@/app/lib/absences-types";
import { hasGlobalAdminRole, hasMasterRole, hasRole } from "@/app/lib/intranet-role-utils";
import type { AccueilBoardKind } from "@/app/lib/accueil-absences-types";

/**
 * Saisie : tout titulaire du module « Absence accueil » (droits modules).
 * Les CPE / direction l’ont par défaut pour consulter ; l’accueil l’active
 * sur les personnes du standard.
 */
export function canDeclareAccueilAbsence(_roles: string[]): boolean {
  return true;
}

/**
 * Lignes personnel OGEC : admin, compta, direction — et le rôle « accueil »
 * (sinon le standard déclare sans jamais revoir la ligne, board « vide »).
 * CPE / surveillants : élèves + profs uniquement.
 */
export function canSeeAccueilBoardKind(kind: AccueilBoardKind, roles: string[]): boolean {
  if (kind === "ogec") {
    return (
      canViewOgecAbsences(roles) ||
      hasGlobalAdminRole(roles) ||
      hasMasterRole(roles) ||
      hasRole(roles, "accueil")
    );
  }
  return true;
}
