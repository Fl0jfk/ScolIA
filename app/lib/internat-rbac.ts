import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import { hasRole } from "@/app/lib/absences-types";
import { hasGlobalAdminRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

export function canAccessInternatModule(roles: string[]) {
  if (hasGlobalAdminRole(roles)) return true;
  return (
    hasRole(roles, "surveillant") ||
    hasRole(roles, "cpe") ||
    hasRole(roles, "administratif") ||
    isAnyDirectionRole(roles)
  );
}

/** Signal dashboard « appel du soir » : réservé à la vie scolaire / éducation. */
export function canSeeInternatRollCallSignal(roles: string[]) {
  return hasRole(roles, "surveillant") || hasRole(roles, "cpe");
}

export function canAccessInternatFromMetadata(meta: unknown) {
  if (isOrgAdminMetadata(meta)) return true;
  return canAccessInternatModule(intranetRolesFromMetadata(meta));
}

export function canManageInternatConfig(roles: string[]) {
  if (hasGlobalAdminRole(roles)) return true;
  return (
    hasRole(roles, "surveillant") ||
    hasRole(roles, "cpe") ||
    hasRole(roles, "administratif") ||
    isAnyDirectionRole(roles)
  );
}

export function isOrgAdminMetadata(meta: unknown) {
  const m = meta as Record<string, unknown> | undefined;
  if (!m) return false;
  if (m.org_admin === true) return true;
  if (m.platform_admin === true) return true;
  return hasGlobalAdminRole(intranetRolesFromMetadata(m));
}
function canMarkRollCall(roles: string[]) {
  return canAccessInternatModule(roles);
}

function canValidateRollCall(roles: string[]) {
  return canAccessInternatModule(roles);
}

export function rolesFromMetadata(meta: unknown): string[] {
  return intranetRolesFromMetadata(meta);
}