import { canViewOgecAbsences } from "@/app/lib/absences-types";
import { hasGlobalAdminRole, hasMasterRole, hasRole } from "@/app/lib/intranet-role-utils";
import type { AccueilBoardKind } from "@/app/lib/accueil-absences-types";

/**
 * Saisie : tout titulaire du module « Absence accueil » (droits modules).
 * Les CPE / direction l’ont par défaut pour consulter ; l’accueil l’active
 * sur les personnes du standard.
 */
export function canDeclareAccueilAbsence(roles: string[]): boolean {
  // NJ-34 (D-053) : accueil, vie scolaire (CPE, surveillants), administratif, directions, admin.
  return (
    hasGlobalAdminRole(roles) ||
    hasMasterRole(roles) ||
    hasRole(roles, "accueil") ||
    hasRole(roles, "cpe") ||
    hasRole(roles, "surveillant") ||
    hasRole(roles, "administratif") ||
    hasRole(roles, "direction")
  );
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
