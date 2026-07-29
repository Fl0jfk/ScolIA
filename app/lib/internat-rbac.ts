import { hasRole } from "@/app/lib/absences-types";
import { hasGlobalAdminRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

export function canAccessInternatModule(roles: string[]) {
  if (hasGlobalAdminRole(roles)) return true;
  return (
    hasRole(roles, "education") ||
    hasRole(roles, "administratif") ||
    hasRole(roles, "directioncollege") ||
    hasRole(roles, "directionlycee")
  );
}

/** Signal dashboard « appel du soir » : réservé à la vie scolaire / éducation. */
export function canSeeInternatRollCallSignal(roles: string[]) {
  return hasRole(roles, "education");
}

export function canAccessInternatFromMetadata(meta: unknown) {
  if (isOrgAdminMetadata(meta)) return true;
  return canAccessInternatModule(intranetRolesFromMetadata(meta));
}

export function canManageInternatConfig(roles: string[]) {
  if (hasGlobalAdminRole(roles)) return true;
  return (
    hasRole(roles, "education") ||
    hasRole(roles, "administratif") ||
    hasRole(roles, "directioncollege") ||
    hasRole(roles, "directionlycee")
  );
}

export function isOrgAdminMetadata(meta: unknown) {
  const m = meta as Record<string, unknown> | undefined;
  if (!m) return false;
  if (m.org_admin === true) return true;
  if (m.platform_admin === true) return true;
  return hasGlobalAdminRole(intranetRolesFromMetadata(m));
}
export function canMarkRollCall(roles: string[]) {
  return canAccessInternatModule(roles);
}

export function canValidateRollCall(roles: string[]) {
  return canAccessInternatModule(roles);
}

export function rolesFromMetadata(meta: unknown): string[] {
  return intranetRolesFromMetadata(meta);
}