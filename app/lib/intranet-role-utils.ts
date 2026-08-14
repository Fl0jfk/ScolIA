/** Utilitaires rôles légers (middleware + catalogue modules, sans dépendance absences). */

export function normRole(s: string): string {
  return String(s)
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, "");
}

export function hasRole(roles: string[], matcher: string): boolean {
  const m = normRole(matcher);
  return roles.some((r) => normRole(r).includes(m));
}

export function hasGlobalAdminRole(roles: string[]): boolean {
  return roles.includes("admin");
}

/** Super-utilisateur plateforme (invisible, tous tenants, config clés API). */
export function hasMasterRole(roles: string[]): boolean {
  return roles.includes("master");
}

/** Administrateur de l'établissement (son tenant uniquement). */
function hasTenantAdminRole(roles: string[]): boolean {
  return hasGlobalAdminRole(roles) || hasMasterRole(roles);
}

export function isHiddenMasterMember(roles: string[]): boolean {
  return hasMasterRole(roles);
}

/** Paramétrage planning domaines : admin org, direction*, administratif. */
/** Profil limité au bot bien-être (pas d'accès dashboard staff). */
export function isEleveOnlyRoleSet(roles: string[]): boolean {
  const visible = roles.filter((r) => r !== "master");
  return visible.length > 0 && visible.every((r) => r === "eleve");
}

export function hasEleveRole(roles: string[]): boolean {
  return hasRole(roles, "eleve");
}

export function canAccessDomainPlanningSettingsFromRoles(roles: string[]): boolean {
  if (hasGlobalAdminRole(roles)) return true;
  return roles.some((r) => {
    const n = normRole(r);
    return n.includes("administratif") || n.includes("direction");
  });
}
