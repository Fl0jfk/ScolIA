/**
 * Ce qu'un compte vaut sur l'établissement visité — règles pures, partagées par le proxy et la
 * lecture de session (routes publiques et supervision comprises).
 */

import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";

export type TenantScopeDeps = {
  /** Le compte est-il rattaché à l'établissement visité ? */
  belongs: () => Promise<boolean>;
  /** Identifiant de l'établissement visité. */
  tenantEtablissementId: () => Promise<string>;
};

/**
 * `null` : le compte n'appartient pas à l'établissement visité, il y est traité comme non connecté.
 * Sinon, établissement dont il faut lire les rôles.
 */
export async function resolveTenantScope(
  input: { homeEtablissementId: string | null; platformAdmin: boolean; tenantSlug: string },
  deps: TenantScopeDeps,
): Promise<{ activeEtablissementId: string | null } | null> {
  if (input.platformAdmin || isPlatformTenantSlug(input.tenantSlug)) {
    return { activeEtablissementId: input.homeEtablissementId };
  }
  if (!(await deps.belongs())) return null;
  return { activeEtablissementId: await deps.tenantEtablissementId() };
}

/**
 * Le drapeau `orgAdmin` est porté par le compte : il ne vaut que sur son établissement d'origine.
 * Ailleurs, seul un rôle `admin` donné sur place compte.
 */
export function scopedOrgAdminFlag(
  flag: boolean,
  homeEtablissementId: string | null,
  activeEtablissementId: string | null,
): boolean {
  return flag && Boolean(homeEtablissementId) && homeEtablissementId === activeEtablissementId;
}
