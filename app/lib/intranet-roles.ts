import { normRole } from "./intranet-role-utils";

/** Slugs stockés dans le directory `publicMetadata.role` (tableau de strings). */
export const INTRANET_ROLE_OPTIONS: { slug: string; label: string }[] = [
  { slug: "admin", label: "Admin (gestion utilisateurs & paramètres)" },
  { slug: "administratif", label: "Administratif" },
  { slug: "professeur", label: "Professeur" },
  { slug: "direction_ecole", label: "Direction école" },
  { slug: "direction_college", label: "Direction collège" },
  { slug: "direction_lycee", label: "Direction lycée" },
  { slug: "direction", label: "Direction" },
  { slug: "comptabilite", label: "Comptabilité" },
  { slug: "maintenance", label: "Maintenance" },
  { slug: "infirmerie", label: "Infirmerie" },
  { slug: "psychologue", label: "Psychologue" },
  { slug: "surveillant", label: "Surveillant" },
  { slug: "internat", label: "Internat (appels du soir & signaux)" },
  { slug: "cpe", label: "CPE" },
  { slug: "parent", label: "Parent" },
  { slug: "eleve", label: "Élève (bot bien-être)" },
];

export const INTRANET_DIRECTION_SLUGS = [
  "direction_ecole",
  "direction_college",
  "direction_lycee",
  "direction",
] as const;

/** Tous les rôles intranet sauf les parents (QR, salons, feuille de semaine…). */
export function intranetRolesExceptParent(): string[] {
  return INTRANET_ROLE_OPTIONS.map((r) => r.slug).filter((s) => s !== "parent");
}

const ALLOWED = new Set(INTRANET_ROLE_OPTIONS.map((r) => r.slug));
const HIDDEN_ROLES = new Set(["master"]);

/** Libellés historiques → slug catalogue (ne retire aucun slug déjà valide). */
function canonicalIntranetRole(raw: string): string | null {
  const s = raw.trim();
  if (!s) return null;
  if (ALLOWED.has(s) || HIDDEN_ROLES.has(s)) return s;
  const n = normRole(s);
  for (const opt of INTRANET_ROLE_OPTIONS) {
    if (normRole(opt.slug) === n || normRole(opt.label) === n) return opt.slug;
  }
  if (n.includes("direction") && n.includes("ecole")) return "direction_ecole";
  if (n.includes("direction") && n.includes("college")) return "direction_college";
  if (n.includes("direction") && n.includes("lycee")) return "direction_lycee";
  if (n === "direction") return "direction";
  // Ancien slug / libellés « éducation / surveillance / vie scolaire » → surveillant
  if (
    n.includes("educat") ||
    n.includes("surveill") ||
    (n.includes("vie") && n.includes("scolaire"))
  ) {
    return "surveillant";
  }
  if (n.includes("internat")) return "internat";
  return null;
}

export function normalizeIntranetRoles(input: unknown): string[] {
  const raw = Array.isArray(input) ? input : typeof input === "string" && input ? [input] : [];
  const out: string[] = [];
  for (const r of raw) {
    const canonical = canonicalIntranetRole(String(r));
    if (!canonical || out.includes(canonical)) continue;
    out.push(canonical);
  }
  return out;
}

export function intranetRolesFromMetadata(meta: unknown): string[] {
  const role = (meta as Record<string, unknown> | undefined)?.role;
  return normalizeIntranetRoles(role);
}

/** Rôles intranet depuis un utilisateur (client ou serveur). */
export function rolesFromUserLike(
  user: { publicMetadata?: unknown } | null | undefined,
): string[] {
  return intranetRolesFromMetadata(user?.publicMetadata);
}

/**
 * Métadonnées publiques depuis le JWT de session.
 * `publicMetadata` n'y est pas par défaut — configurer le jeton de session
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

/** Rôles intranet depuis les claims JWT (jeton de session personnalisé). */
export function intranetRolesFromSessionClaims(
  claims: Record<string, unknown> | null | undefined,
): string[] {
  const fromMeta = intranetRolesFromMetadata(publicMetadataFromSessionClaims(claims));
  if (fromMeta.length > 0) return fromMeta;
  return normalizeIntranetRoles(claims?.role);
}

export {hasGlobalAdminRole, hasMasterRole, isHiddenMasterMember} from "./intranet-role-utils";
