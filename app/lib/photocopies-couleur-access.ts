import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  directionRolesMatchEstablishmentRef,
  isAnyDirectionRole,
} from "@/app/lib/establishment-catalog";
import { hasGlobalAdminRole, hasMasterRole, hasRole } from "@/app/lib/intranet-role-utils";

type PhotocopiesRecordLike = {
  etablissement: string;
  createdBy: { userId: string };
  status?: string;
};

export function getPhotocopiesRoleFlags(roles: string[]) {
  return {
    isDirection: isAnyDirectionRole(roles),
    isAdministratif: hasRole(roles, "administratif"),
    isProfesseur: hasRole(roles, "professeur"),
    isEducation: hasRole(roles, "surveillant") || hasRole(roles, "cpe"),
  };
}

export function canCreatePhotocopiesDemand(roles: string[]) {
  const f = getPhotocopiesRoleFlags(roles);
  return (
    f.isProfesseur ||
    f.isAdministratif ||
    f.isEducation ||
    hasGlobalAdminRole(roles) ||
    hasMasterRole(roles) ||
    roles.includes("admin")
  );
}

/** Administratif, comptabilité, direction et admin établissement — déposer une demande pour un enseignant. */
export function canDeclarePhotocopiesOnBehalf(roles: string[]) {
  const f = getPhotocopiesRoleFlags(roles);
  return (
    f.isAdministratif ||
    hasRole(roles, "comptabilite") ||
    f.isDirection ||
    hasGlobalAdminRole(roles) ||
    hasMasterRole(roles) ||
    roles.includes("admin")
  );
}

export function canManagePhotocopiesDemand(
  rec: PhotocopiesRecordLike,
  roles: string[],
  establishments: Establishment[] = [],
  userId?: string | null,
) {
  return directionRolesMatchEstablishmentRef(roles, rec.etablissement, establishments, userId);
}

export function canViewPhotocopiesDemand(
  rec: PhotocopiesRecordLike,
  userId: string,
  roles: string[],
  establishments: Establishment[] = [],
  opts?: { isOpsHandler?: boolean },
) {
  if (rec.createdBy.userId === userId) return true;
  if (opts?.isOpsHandler) {
    // File impressions : acceptées (à imprimer) + déjà marquées prêtes
    return rec.status === "ACCEPTEE" || rec.status === "PRETE";
  }
  return canManagePhotocopiesDemand(rec, roles, establishments, userId);
}

/** Réceptionnaire impressions : marquer ACCEPTEE → PRETE. */
export function canProcessPhotocopiesOps(
  roles: string[],
  isOpsHandler: boolean,
) {
  if (isOpsHandler) return true;
  return (
    hasGlobalAdminRole(roles) ||
    hasMasterRole(roles) ||
    roles.includes("admin")
  );
}
