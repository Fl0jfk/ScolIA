import "server-only";

import { listMembersFromDb } from "@/app/lib/members-db";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { getTenant } from "@/app/lib/tenant-context";
import { isDatabaseConfigured } from "@/db/index";
import { intranetRolesFromMetadata, normalizeIntranetRoles } from "@/app/lib/intranet-roles";

export type DirectoryMemberRow = {
  /** Id Better-Auth (clé droits modules). */
  userId?: string;
  externalUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  roles: string[];
  pending: boolean;
  /** Compte déjà passé par MFA (activation terminée). */
  mfaEnabled?: boolean;
  createdAt: string;
  updatedAt: string;
};

function displayName(u: {
  firstName?: string | null;
  lastName?: string | null;
  email: string;
}): string {
  const n = `${u.firstName ?? ""} ${u.lastName ?? ""}`.trim();
  return n || u.email;
}

/** Liste les membres depuis PostgreSQL (Better-Auth). */
export async function listDirectoryMembers(): Promise<DirectoryMemberRow[]> {
  if (!isDatabaseConfigured()) return [];
  try {
    const tenant = await getTenant();
    const etablissementId = await ensureEtablissementFromTenant(tenant);
    return listMembersFromDb(etablissementId);
  } catch (error) {
    console.error("[listDirectoryMembers]", error);
    return [];
  }
}

export async function getDirectoryUserRoles(externalUserId: string): Promise<string[]> {
  if (!isDatabaseConfigured()) return [];
  const tenant = await getTenant();
  const etablissementId = await ensureEtablissementFromTenant(tenant);
  const { findDbUserByExternalId, getDbUserRoles } = await import("@/app/lib/members-db");
  const dbUser = await findDbUserByExternalId(etablissementId, externalUserId);
  if (!dbUser) return [];
  return getDbUserRoles(dbUser.id, etablissementId);
}

export async function syncDirectoryUserRoles(_externalUserId: string, _roles: string[]): Promise<void> {
  // No-op : les rôles sont en PostgreSQL (setUserRolesInDb).
}

export function memberRowFromDirectoryUser(u: {
  id: string;
  firstName?: string | null;
  lastName?: string | null;
  emailAddresses: { id: string; emailAddress: string }[];
  primaryEmailAddressId: string | null;
  publicMetadata: unknown;
  createdAt: number;
  updatedAt: number;
}): DirectoryMemberRow {
  const email =
    u.emailAddresses.find((e) => e.id === u.primaryEmailAddressId)?.emailAddress ??
    u.emailAddresses[0]?.emailAddress ??
    "";
  return {
    externalUserId: u.id,
    email,
    firstName: u.firstName ?? undefined,
    lastName: u.lastName ?? undefined,
    displayName: displayName({ firstName: u.firstName, lastName: u.lastName, email }),
    roles: intranetRolesFromMetadata(u.publicMetadata),
    pending: false,
    createdAt: new Date(u.createdAt).toISOString(),
    updatedAt: new Date(u.updatedAt).toISOString(),
  };
}

export { normalizeIntranetRoles, displayName };
