import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  directionRolesMatchEstablishmentRef,
  isAnyDirectionRole,
} from "@/app/lib/establishment-catalog";
import { hasRole } from "@/app/lib/intranet-role-utils";

export type HseEtablissement = string;
export type HseStatus = "EN_ATTENTE" | "ACCEPTEE" | "REFUSEE" | "ANNULEE";

export type HseRecordLike = {
  status: HseStatus | string;
  etablissement: HseEtablissement;
  createdBy: { userId: string };
};

export function getHseRoleFlags(roles: string[]) {
  return {
    isDirection: isAnyDirectionRole(roles),
    isProfesseur: hasRole(roles, "professeur"),
    isAdministratif: hasRole(roles, "administratif"),
  };
}

export function canCreateHseDemand(roles: string[]) {
  return getHseRoleFlags(roles).isProfesseur;
}

export function canManageHseDemand(
  rec: HseRecordLike,
  roles: string[],
  establishments: Establishment[] = [],
  userId?: string | null,
) {
  return directionRolesMatchEstablishmentRef(roles, rec.etablissement, establishments, userId);
}

export function canViewHseDemand(
  rec: HseRecordLike,
  userId: string,
  roles: string[],
  establishments: Establishment[] = [],
) {
  if (rec.createdBy.userId === userId) return true;
  return canManageHseDemand(rec, roles, establishments, userId);
}

export function canAccessHseModule(roles: string[]) {
  const f = getHseRoleFlags(roles);
  return f.isProfesseur || f.isDirection || f.isAdministratif;
}
