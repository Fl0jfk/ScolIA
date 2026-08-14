import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  directionRolesMatchEstablishmentRef,
  isAnyDirectionRole,
} from "@/app/lib/establishment-catalog";
import { hasRole } from "@/app/lib/intranet-role-utils";

type PhotocopiesRecordLike = {
  etablissement: string;
  createdBy: { userId: string };
};

export function getPhotocopiesRoleFlags(roles: string[]) {
  return {
    isDirection: isAnyDirectionRole(roles),
    isAdministratif: hasRole(roles, "administratif"),
    isProfesseur: hasRole(roles, "professeur"),
    isEducation: hasRole(roles, "education") || hasRole(roles, "cpe"),
  };
}

export function canCreatePhotocopiesDemand(roles: string[]) {
  const f = getPhotocopiesRoleFlags(roles);
  return f.isProfesseur || f.isAdministratif || f.isEducation;
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
) {
  if (rec.createdBy.userId === userId) return true;
  return canManagePhotocopiesDemand(rec, roles, establishments, userId);
}
