import { intranetRolesFromMetadata, rolesFromUserLike } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";

export function userHasAdministratifRoleFromMetadata(
  publicMetadata?: Record<string, unknown> | null,
): boolean {
  return hasRole(intranetRolesFromMetadata(publicMetadata), "administratif");
}

/** Réattribuer le créateur d'un dossier voyage (administratif ou admin org). */
export function canReassignTravelsOwner(
  user: { publicMetadata?: Record<string, unknown> | null } | null | undefined,
): boolean {
  const roles = intranetRolesFromMetadata(user?.publicMetadata);
  return hasRole(roles, "administratif") || hasGlobalAdminRole(roles);
}

export function userHasComptaRoleFromMetadata(
  publicMetadata?: Record<string, unknown> | null,
): boolean {
  return hasRole(rolesFromUserLike({ publicMetadata }), "comptabilite");
}

export function userHasAdministratifRole(user: {
  publicMetadata?: Record<string, unknown> | null;
} | null | undefined): boolean {
  return userHasAdministratifRoleFromMetadata(user?.publicMetadata);
}
