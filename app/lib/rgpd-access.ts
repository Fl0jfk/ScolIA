import { hasRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";

/** Rôles autorisés sur le module Conformité RGPD (tuile, routes, API). */
export const RGPD_MODULE_ROLES = [
  ...INTRANET_DIRECTION_SLUGS,
  "administratif",
  "comptabilite",
] as const;

function isDirectionRole(roles: string[]): boolean {
  return INTRANET_DIRECTION_SLUGS.some((slug) => hasRole(roles, slug));
}

export function canAccessRgpdModule(roles: string[]): boolean {
  return RGPD_MODULE_ROLES.some((slug) => hasRole(roles, slug));
}

function canManageRgpdWorkspace(roles: string[]): boolean {
  return canAccessRgpdModule(roles);
}
