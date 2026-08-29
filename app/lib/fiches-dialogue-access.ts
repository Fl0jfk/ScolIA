import "server-only";

import { hasGlobalAdminRole, INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";

export function canManageFichesDialogue(roles: string[], opts?: { orgAdmin?: boolean }): boolean {
  if (opts?.orgAdmin) return true;
  if (hasGlobalAdminRole(roles)) return true;
  if (INTRANET_DIRECTION_SLUGS.some((s) => roles.includes(s))) return true;
  if (hasRole(roles, "administratif") || hasRole(roles, "admin")) return true;
  return false;
}

export function canConseilFichesDialogue(roles: string[], opts?: { orgAdmin?: boolean }): boolean {
  if (canManageFichesDialogue(roles, opts)) return true;
  if (hasRole(roles, "professeur") || hasRole(roles, "cpe")) return true;
  return false;
}

export function canViewFichesDialogue(roles: string[], opts?: { orgAdmin?: boolean }): boolean {
  if (canConseilFichesDialogue(roles, opts)) return true;
  if (hasRole(roles, "parent") || hasRole(roles, "surveillant")) return true;
  return false;
}
