/** Détecte le profil élève bien-être (bot d'écoute, pas Nico institutionnel). */

const STAFF_ROLE_SLUGS = new Set([
  "admin",
  "administratif",
  "professeur",
  "direction_ecole",
  "direction_college",
  "direction_lycee",
  "comptabilite",
  "maintenance",
  "infirmerie",
  "education",
  "cpe",
  "parent",
  "master",
]);

export function intranetRolesFromUnknown(meta: unknown): string[] {
  const role = (meta as Record<string, unknown> | undefined)?.role;
  if (Array.isArray(role)) return role.map(String);
  if (typeof role === "string" && role.trim()) return [role.trim()];
  return [];
}

/** Élève sans rôle staff → agent bien-être dans la bulle IA. */
export function isEleveBienEtreProfile(roles: string[]): boolean {
  if (!roles.includes("eleve")) return false;
  return !roles.some((r) => STAFF_ROLE_SLUGS.has(r));
}
