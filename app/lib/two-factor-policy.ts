/** Rôles famille / portail — pas de 2FA TOTP obligatoire pour l’instant. */
const FAMILY_ONLY_ROLES = new Set(["parent", "eleve"]);

/**
 * Indique si le compte doit activer la 2FA (TOTP).
 * Obligatoire pour tout le personnel intranet ; exclus uniquement les comptes
 * dont les rôles sont purement `parent` et/ou `eleve`.
 */
export function roleRequiresTwoFactor(opts: {
  platformAdmin: boolean;
  orgAdmin: boolean;
  roles: string[];
}): boolean {
  if (opts.platformAdmin || opts.orgAdmin) return true;

  const roles = opts.roles.filter(Boolean);
  if (roles.length === 0) return true;

  const hasStaffRole = roles.some((role) => !FAMILY_ONLY_ROLES.has(role));
  return hasStaffRole;
}
