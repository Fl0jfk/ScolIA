import { hasGlobalAdminRole, hasMasterRole, intranetRolesFromMetadata } from "@/app/lib/intranet-roles";

export function isPlatformMasterFromPublicMetadata(meta: unknown): boolean {
  return hasMasterRole(intranetRolesFromMetadata(meta));
}

function isTenantAdminFromPublicMetadata(meta: unknown): boolean {
  const m = meta as Record<string, unknown> | undefined;
  if (!m) return false;
  if (m.org_admin === true) return true;
  return hasGlobalAdminRole(intranetRolesFromMetadata(m));
}

/** Accès aux modules orgAdminOnly et APIs admin tenant. */
export function isOrgAdminFromPublicMetadata(meta: unknown): boolean {
  const m = meta as Record<string, unknown> | undefined;
  if (!m) return false;
  if (isPlatformMasterFromPublicMetadata(m)) return true;
  if (m.platform_admin === true) return true;
  return isTenantAdminFromPublicMetadata(m);
}
