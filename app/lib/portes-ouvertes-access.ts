import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import {
  hasGlobalAdminRole,
  hasMasterRole,
  hasRole,
} from "@/app/lib/intranet-role-utils";

/**
 * Paramétrage Accueil PO (équipe créneaux) : direction + admin uniquement.
 * Le planning reste ouvert à administratif / accueil quand l’outil est activé.
 */
export function canManagePortesOuvertesParametrage(
  roles: readonly string[],
  isOrgAdmin = false,
): boolean {
  const roleList = [...roles];
  if (isOrgAdmin || hasGlobalAdminRole(roleList) || hasMasterRole(roleList)) return true;
  if (hasRole(roleList, "admin")) return true;
  return INTRANET_DIRECTION_SLUGS.some((slug) => hasRole(roleList, slug));
}
