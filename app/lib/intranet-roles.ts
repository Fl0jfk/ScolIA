/** Slugs stockés dans Clerk `publicMetadata.role` (tableau de strings). */
export const INTRANET_ROLE_OPTIONS: { slug: string; label: string }[] = [
  { slug: "admin", label: "Admin (gestion utilisateurs & paramètres)" },
  { slug: "administratif", label: "Administratif" },
  { slug: "professeur", label: "Professeur" },
  { slug: "direction_ecole", label: "Direction école" },
  { slug: "direction_college", label: "Direction collège" },
  { slug: "direction_lycee", label: "Direction lycée" },
  { slug: "comptabilite", label: "Comptabilité" },
  { slug: "maintenance", label: "Maintenance" },
  { slug: "infirmerie", label: "Infirmerie" },
  { slug: "psychologue", label: "Psychologue" },
  { slug: "education", label: "Éducation / surveillance (vie scolaire)" },
  { slug: "cpe", label: "CPE" },
  { slug: "parent", label: "Parent" },
  { slug: "eleve", label: "Élève (bot bien-être)" },
];

export const INTRANET_DIRECTION_SLUGS = [
  "direction_ecole",
  "direction_college",
  "direction_lycee",
] as const;

/** Tous les rôles intranet sauf les parents (QR, salons, feuille de semaine…). */
export function intranetRolesExceptParent(): string[] {
  return INTRANET_ROLE_OPTIONS.map((r) => r.slug).filter((s) => s !== "parent");
}

const ALLOWED = new Set(INTRANET_ROLE_OPTIONS.map((r) => r.slug));
const HIDDEN_ROLES = new Set(["master"]);

export function normalizeIntranetRoles(input: unknown): string[] {
  const raw = Array.isArray(input) ? input : typeof input === "string" && input ? [input] : [];
  const out: string[] = [];
  for (const r of raw) {
    const s = String(r).trim();
    if (!s || out.includes(s)) continue;
    if (ALLOWED.has(s) || HIDDEN_ROLES.has(s)) out.push(s);
  }
  return out;
}

export function intranetRolesFromMetadata(meta: unknown): string[] {
  const role = (meta as Record<string, unknown> | undefined)?.role;
  return normalizeIntranetRoles(role);
}

/**
 * Métadonnées publiques depuis le JWT de session Clerk.
 * `publicMetadata` n'y est pas par défaut — configurer le session token Clerk
 * ou laisser le middleware recharger via l'API (repli).
 */
export function publicMetadataFromSessionClaims(
  claims: Record<string, unknown> | null | undefined,
): Record<string, unknown> | undefined {
  if (!claims) return undefined;

  const nested = (claims.publicMetadata ?? claims.public_metadata) as
    | Record<string, unknown>
    | undefined;
  if (nested && typeof nested === "object") return nested;

  const metadata = claims.metadata as Record<string, unknown> | undefined;
  if (metadata && typeof metadata === "object") return metadata;

  if ("role" in claims || "org_admin" in claims || "platform_admin" in claims) {
    return {
      role: claims.role,
      org_admin: claims.org_admin,
      platform_admin: claims.platform_admin,
    };
  }

  return undefined;
}

/** Rôles intranet depuis les claims JWT (session token Clerk personnalisé). */
export function intranetRolesFromSessionClaims(
  claims: Record<string, unknown> | null | undefined,
): string[] {
  const fromMeta = intranetRolesFromMetadata(publicMetadataFromSessionClaims(claims));
  if (fromMeta.length > 0) return fromMeta;
  return normalizeIntranetRoles(claims?.role);
}

export { hasGlobalAdminRole, hasMasterRole, hasTenantAdminRole, isHiddenMasterMember } from "./intranet-role-utils";
