import { intranetRolesFromMetadata, rolesFromUserLike } from "@/app/lib/intranet-roles";
import { isOrgAdminFromPublicMetadata } from "@/app/lib/intranet-auth-metadata";
import { hasGlobalAdminRole, hasRole } from "@/app/lib/intranet-role-utils";

export function userHasAdministratifRoleFromMetadata(
  publicMetadata?: Record<string, unknown> | null,
): boolean {
  return hasRole(intranetRolesFromMetadata(publicMetadata), "administratif");
}

/**
 * Staff voyage « admin » : rôle administratif OU administrateur général
 * (rôle `admin`, flag org_admin, etc.).
 */
export function canReassignTravelsOwner(
  user: { publicMetadata?: Record<string, unknown> | null } | null | undefined,
): boolean {
  const meta = user?.publicMetadata;
  if (isOrgAdminFromPublicMetadata(meta)) return true;
  const roles = intranetRolesFromMetadata(meta);
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
