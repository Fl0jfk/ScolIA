import { hasGlobalAdminRole, INTRANET_ROLE_OPTIONS, normalizeIntranetRoles } from "@/app/lib/intranet-roles";
import { listDirectoryMembers, type DirectoryMemberRow } from "@/app/lib/directory-members";
import {
  inferCategoryFromRoles,
  type PersonnelCategory,
  type PersonnelIndexEntry,
} from "@/app/lib/personnel-types";

/** Rôles OGEC typiques (hors professeur) — utilisé pour suggestion de catégorie. */
const RH_OGEC_LEGACY_AUTH_ROLES = [
  "administratif",
  "surveillant",
  "cpe",
  "comptabilite",
  "maintenance",
  "direction_ecole",
  "direction_college",
  "direction_lycee",
  "infirmerie",
  "admin",
] as const;

type RhDirectoryCandidate = DirectoryMemberRow & {
  suggestedCategory: PersonnelCategory;
  roleLabel: string;
};

const ROLE_LABELS: Record<string, string> = Object.fromEntries(
  INTRANET_ROLE_OPTIONS.map((r) => [r.slug, r.label]),
);

function directoryRoleLabel(slug: string): string {
  return ROLE_LABELS[slug] || slug;
}

/** Exclut uniquement les comptes dont le seul rôle est professeur. */
function isProfesseurOnly(roles: string[]): boolean {
  const normalized = normalizeIntranetRoles(roles);
  return normalized.length > 0 && normalized.every((r) => r === "professeur");
}

export function suggestPersonnelCategoryFromRoles(roles: string[]): PersonnelCategory {
  const inferred = inferCategoryFromRoles(roles);
  if (inferred) return inferred;
  return "administratif";
}

function directoryRolesForPersonnelCategory(category: PersonnelCategory): string[] {
  return [category];
}

function formatDirectoryRolesLabel(roles: string[]): string {
  const normalized = normalizeIntranetRoles(roles);
  if (normalized.length === 0) return "Rôle non renseigné dans le directory";
  return normalized.map(directoryRoleLabel).join(" · ");
}

function filterRhDirectoryCandidates(
  members: DirectoryMemberRow[],
  existingIndex: PersonnelIndexEntry[],
): RhDirectoryCandidate[] {
  const linkedIds = new Set(existingIndex.map((e) => e.externalUserId).filter(Boolean));
  const linkedEmails = new Set(existingIndex.map((e) => e.email.trim().toLowerCase()));

  return members
    .filter((m) => {
      if (!m.email?.trim()) return false;
      if (isProfesseurOnly(m.roles)) return false;
      if (m.externalUserId && linkedIds.has(m.externalUserId)) return false;
      if (linkedEmails.has(m.email.trim().toLowerCase())) return false;
      return true;
    })
    .map((m) => ({
      ...m,
      suggestedCategory: suggestPersonnelCategoryFromRoles(m.roles),
      roleLabel: formatDirectoryRolesLabel(m.roles),
    }));
}

export async function listRhDirectoryCandidates(existingIndex: PersonnelIndexEntry[]): Promise<RhDirectoryCandidate[]> {
  const members = await listDirectoryMembers();
  return filterRhDirectoryCandidates(members, existingIndex);
}

export async function ensureDirectoryUserForPersonnel(input: {
  email: string;
  firstName: string;
  lastName: string;
  category: PersonnelCategory;
  /** Surcharge des rôles (ex. professeur pour une entrée enseignant). */
  roles?: string[];
}): Promise<{ externalUserId: string | null; mode: "existing" | "invitation"; pending: boolean }> {
  const email = input.email.trim().toLowerCase();
  const roles = input.roles?.length
    ? input.roles
    : directoryRolesForPersonnelCategory(input.category);

  const existing = await findDirectoryMemberByEmail(email);
  if (existing?.externalUserId) {
    const { setUserRolesInDb, syncUserAdminFlagsInDb } = await import("@/app/lib/auth-roles-db");
    const { ensureEtablissementFromTenant } = await import("@/app/lib/etablissement-db");
    const { getTenant } = await import("@/app/lib/tenant-context");
    const { findDbUserByExternalId } = await import("@/app/lib/members-db");
    const tenant = await getTenant();
    const etablissementId = await ensureEtablissementFromTenant(tenant);
    const dbUser = await findDbUserByExternalId(etablissementId, existing.externalUserId);
    if (dbUser) {
      const merged = [...new Set([...existing.roles, ...roles])];
      await setUserRolesInDb(dbUser.id, etablissementId, merged);
      await syncUserAdminFlagsInDb(dbUser.id, merged);
    }
    return { externalUserId: existing.externalUserId, mode: "existing", pending: existing.pending };
  }

  const { getDb, isDatabaseConfigured } = await import("@/db/index");
  const { user } = await import("@/db/schema");
  const { ensureEtablissementFromTenant } = await import("@/app/lib/etablissement-db");
  const { getTenant } = await import("@/app/lib/tenant-context");
  const { setUserRolesInDb, syncUserAdminFlagsInDb } = await import("@/app/lib/auth-roles-db");
  if (!isDatabaseConfigured()) {
    throw new Error("DATABASE_URL requise pour créer un compte personnel.");
  }
  const tenant = await getTenant();
  const etablissementId = await ensureEtablissementFromTenant(tenant);
  const id = crypto.randomUUID();
  const db = getDb();
  await db.insert(user).values({
    id,
    email,
    name: `${input.firstName} ${input.lastName}`.trim() || email,
    firstName: input.firstName || null,
    lastName: input.lastName || null,
    emailVerified: false,
    etablissementId,
    orgAdmin: hasGlobalAdminRole(roles),
  });
  await setUserRolesInDb(id, etablissementId, roles);
  await syncUserAdminFlagsInDb(id, roles);
  return { externalUserId: id, mode: "invitation", pending: true };
}

export async function findDirectoryMemberByEmail(email: string): Promise<DirectoryMemberRow | null> {
  const normalized = email.trim().toLowerCase();
  if (!normalized) return null;
  const members = await listDirectoryMembers();
  return members.find((m) => m.email.trim().toLowerCase() === normalized) || null;
}

export async function getDirectoryMemberById(externalUserId: string): Promise<DirectoryMemberRow | null> {
  const members = await listDirectoryMembers();
  return members.find((m) => m.externalUserId === externalUserId) || null;
}

/** @deprecated Utiliser isProfesseurOnly — conservé pour compatibilité interne */
function hasRhEligibleDirectoryRole(roles: string[]): boolean {
  if (isProfesseurOnly(roles)) return false;
  const normalized = normalizeIntranetRoles(roles);
  if (normalized.length === 0) return true;
  return !normalized.every((r) => r === "professeur");
}
