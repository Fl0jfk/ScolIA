import { hasGlobalAdminRole, INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";

/** Même règle que `canEditStructure` de /api/eleves/[id]/dossier (admin, direction, administratif). */
export function canManageFoyersRoles(
  roles: string[],
  opts: { orgAdmin?: boolean; platformAdmin?: boolean },
): boolean {
  return Boolean(
    opts.orgAdmin ||
      opts.platformAdmin ||
      roles.includes("admin") ||
      hasGlobalAdminRole(roles) ||
      INTRANET_DIRECTION_SLUGS.some((slug) => roles.includes(slug)) ||
      hasRole(roles, "administratif"),
  );
}
