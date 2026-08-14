import { getClerkClientForTenant } from "@/app/lib/tenant-clerk";
import { hasGlobalAdminRole, INTRANET_ROLE_OPTIONS, normalizeIntranetRoles } from "@/app/lib/intranet-roles";
import { listClerkMembers, memberRowFromClerkUser, type ClerkMemberRow } from "@/app/lib/clerk-users";
import {
  inferCategoryFromRoles,
  type PersonnelCategory,
  type PersonnelIndexEntry,
} from "@/app/lib/personnel-types";

/** Rôles OGEC typiques (hors professeur) — utilisé pour suggestion de catégorie. */
const RH_OGEC_CLERK_ROLES = [
  "administratif",
  "education",
  "cpe",
  "comptabilite",
  "maintenance",
  "direction_ecole",
  "direction_college",
  "direction_lycee",
  "infirmerie",
  "admin",
] as const;

type RhClerkCandidate = ClerkMemberRow & {
  suggestedCategory: PersonnelCategory;
  roleLabel: string;
};

const ROLE_LABELS: Record<string, string> = Object.fromEntries(
  INTRANET_ROLE_OPTIONS.map((r) => [r.slug, r.label]),
);

function clerkRoleLabel(slug: string): string {
  return ROLE_LABELS[slug] || slug;
}

/** Exclut uniquement les comptes dont le seul rôle est professeur. */
function isProfesseurOnly(roles: string[]): boolean {
  const normalized = normalizeIntranetRoles(roles);
  return normalized.length > 0 && normalized.every((r) => r === "professeur");
}

export function suggestPersonnelCategoryFromClerkRoles(roles: string[]): PersonnelCategory {
  const inferred = inferCategoryFromRoles(roles);
  if (inferred) return inferred;
  return "administratif";
}

function clerkRolesForPersonnelCategory(category: PersonnelCategory): string[] {
  return [category];
}

function formatClerkRolesLabel(roles: string[]): string {
  const normalized = normalizeIntranetRoles(roles);
  if (normalized.length === 0) return "Rôle non renseigné dans Clerk";
  return normalized.map(clerkRoleLabel).join(" · ");
}

function filterRhClerkCandidates(
  members: ClerkMemberRow[],
  existingIndex: PersonnelIndexEntry[],
): RhClerkCandidate[] {
  const linkedIds = new Set(existingIndex.map((e) => e.clerkUserId).filter(Boolean));
  const linkedEmails = new Set(existingIndex.map((e) => e.email.trim().toLowerCase()));

  return members
    .filter((m) => {
      if (!m.email?.trim()) return false;
      if (isProfesseurOnly(m.roles)) return false;
      if (m.clerkUserId && linkedIds.has(m.clerkUserId)) return false;
      if (linkedEmails.has(m.email.trim().toLowerCase())) return false;
      return true;
    })
    .map((m) => ({
      ...m,
      suggestedCategory: suggestPersonnelCategoryFromClerkRoles(m.roles),
      roleLabel: formatClerkRolesLabel(m.roles),
    }));
}

export async function listRhClerkCandidates(existingIndex: PersonnelIndexEntry[]): Promise<RhClerkCandidate[]> {
  const members = await listClerkMembers();
  return filterRhClerkCandidates(members, existingIndex);
}

export async function ensureClerkUserForPersonnel(input: {
  email: string;
  firstName: string;
  lastName: string;
  category: PersonnelCategory;
  /** Surcharge des rôles Clerk (ex. professeur pour une entrée enseignant). */
  roles?: string[];
}): Promise<{ clerkUserId: string | null; mode: "existing" | "invitation"; pending: boolean }> {
  const email = input.email.trim().toLowerCase();
  const roles = input.roles?.length
    ? input.roles
    : clerkRolesForPersonnelCategory(input.category);
  const client = await getClerkClientForTenant();

  const existing = await client.users.getUserList({ emailAddress: [email], limit: 1 });
  const clerkUser = existing.data?.[0];

  if (clerkUser) {
    const currentRoles = normalizeIntranetRoles(
      (clerkUser.publicMetadata as Record<string, unknown> | undefined)?.role,
    );
    const merged = [...new Set([...currentRoles, ...roles])];
    await client.users.updateUser(clerkUser.id, {
      ...(input.firstName ? { firstName: input.firstName } : {}),
      ...(input.lastName ? { lastName: input.lastName } : {}),
      publicMetadata: {
        ...(clerkUser.publicMetadata as object),
        role: merged,
        org_admin: hasGlobalAdminRole(merged),
      },
    });
    return { clerkUserId: clerkUser.id, mode: "existing", pending: false };
  }

  await client.invitations.createInvitation({
    emailAddress: email,
    publicMetadata: { role: roles },
  });

  return { clerkUserId: null, mode: "invitation", pending: true };
}

export async function findClerkMemberByEmail(email: string): Promise<ClerkMemberRow | null> {
  const normalized = email.trim().toLowerCase();
  if (!normalized) return null;
  const members = await listClerkMembers();
  return members.find((m) => m.email.trim().toLowerCase() === normalized) || null;
}

export async function getClerkMemberById(clerkUserId: string): Promise<ClerkMemberRow | null> {
  const client = await getClerkClientForTenant();
  try {
    const user = await client.users.getUser(clerkUserId);
    return memberRowFromClerkUser(user);
  } catch {
    return null;
  }
}

/** @deprecated Utiliser isProfesseurOnly — conservé pour compatibilité interne */
function hasRhEligibleClerkRole(roles: string[]): boolean {
  if (isProfesseurOnly(roles)) return false;
  const normalized = normalizeIntranetRoles(roles);
  if (normalized.length === 0) return true;
  return !normalized.every((r) => r === "professeur");
}
