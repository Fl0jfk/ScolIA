import { hasRole } from "@/app/lib/intranet-role-utils";

export type HseEtablissement = "École" | "Collège" | "Lycée";
export type HseStatus = "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE" | "ANNULEE";

export type HseRecordLike = {
  status: HseStatus | string;
  etablissement: HseEtablissement;
  createdBy: { userId: string };
};

const norm = (s: string) =>
  String(s || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, "");

export function getHseRoleFlags(roles: string[]) {
  const n = roles.map(norm);
  return {
    isDirectionEcole: n.some((r) => r.includes("direction") && r.includes("ecole")),
    isDirectionCollege: n.some((r) => r.includes("direction") && r.includes("college")),
    isDirectionLycee: n.some((r) => r.includes("direction") && r.includes("lycee")),
    isProfesseur: n.some((r) => r.includes("professeur")),
    isAdministratif: hasRole(roles, "administratif"),
  };
}

export function canCreateHseDemand(roles: string[]) {
  return getHseRoleFlags(roles).isProfesseur;
}

/** Direction de l’établissement concerné uniquement. */
export function canManageHseDemand(rec: HseRecordLike, roles: string[]) {
  const f = getHseRoleFlags(roles);
  if (rec.etablissement === "École") return f.isDirectionEcole;
  if (rec.etablissement === "Collège") return f.isDirectionCollege;
  if (rec.etablissement === "Lycée") return f.isDirectionLycee;
  return false;
}

/**
 * Visibilité HSE :
 * - le demandeur voit uniquement ses demandes (en attente ou traitées) ;
 * - la direction voit les demandes de son établissement (à traiter et traitées) ;
 * - personne d’autre (pas l’administratif).
 */
export function canViewHseDemand(rec: HseRecordLike, userId: string, roles: string[]) {
  if (rec.createdBy.userId === userId) return true;
  return canManageHseDemand(rec, roles);
}

export function canAccessHseModule(roles: string[]) {
  const f = getHseRoleFlags(roles);
  return (
    f.isProfesseur ||
    f.isDirectionEcole ||
    f.isDirectionCollege ||
    f.isDirectionLycee
  );
}

/** @deprecated Plus d’accès administratif aux HSE. */
function canViewAcceptedHseAsAdministratif(_rec: HseRecordLike, _roles: string[]) {
  return false;
}

/** @deprecated Plus d’accès administratif aux HSE. */
function isHseAdministratifViewer(_roles: string[]) {
  return false;
}
