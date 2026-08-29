import "server-only";

import { eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user, userRole } from "@/db/schema";
import type { DirectoryMemberRow } from "@/app/lib/directory-members";
import { listUserRolesBatchFromDb, listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { ensureUserInvitationSentAtColumn } from "@/app/lib/user-invitation-sent";

export async function listMembersFromDb(etablissementId: string): Promise<DirectoryMemberRow[]> {
  if (!isDatabaseConfigured()) return [];
  await ensureUserInvitationSentAtColumn();
  const db = getDb();
  const users = await db.select().from(user).where(eq(user.etablissementId, etablissementId));
  const rolesByUserId = await listUserRolesBatchFromDb(
    users.map((u) => u.id),
    etablissementId,
  );
  const rows: DirectoryMemberRow[] = [];
  for (const u of users) {
    const roles = rolesByUserId.get(u.id) ?? [];
    rows.push({
      userId: u.id,
      externalUserId: u.externalUserId ?? u.id,
      email: u.email,
      firstName: u.firstName ?? undefined,
      lastName: u.lastName ?? undefined,
      displayName: u.name,
      roles,
      pending: !u.emailVerified || u.mustChangePassword || !u.twoFactorEnabled,
      mfaEnabled: u.twoFactorEnabled,
      invitationSentAt: u.invitationSentAt ? u.invitationSentAt.toISOString() : null,
      createdAt: u.createdAt.toISOString(),
      updatedAt: u.updatedAt.toISOString(),
    });
  }
  return rows.sort((a, b) => (a.displayName ?? a.email).localeCompare(b.displayName ?? b.email, "fr"));
}

export async function mergeMemberSources(
  memberRows: DirectoryMemberRow[],
  dbRows: DirectoryMemberRow[],
): Promise<DirectoryMemberRow[]> {
  const byEmail = new Map<string, DirectoryMemberRow>();
  for (const row of memberRows) {
    byEmail.set(row.email.toLowerCase(), row);
  }
  for (const row of dbRows) {
    const key = row.email.toLowerCase();
    const existing = byEmail.get(key);
    if (!existing) {
      byEmail.set(key, row);
      continue;
    }
    byEmail.set(key, {
      ...existing,
      roles: row.roles.length ? row.roles : existing.roles,
      externalUserId: existing.externalUserId || row.externalUserId,
    });
  }
  return [...byEmail.values()].sort((a, b) =>
    (a.displayName ?? a.email).localeCompare(b.displayName ?? b.email, "fr"),
  );
}

export async function getDbUserRoles(userId: string, etablissementId: string): Promise<string[]> {
  return listUserRolesFromDb(userId, etablissementId);
}

export async function findDbUserByExternalId(
  etablissementId: string,
  externalUserId: string,
): Promise<(typeof user.$inferSelect) | null> {
  if (!isDatabaseConfigured()) return null;
  const db = getDb();
  const rows = await db.select().from(user).where(eq(user.etablissementId, etablissementId));
  return rows.find((u) => u.externalUserId === externalUserId || u.id === externalUserId) ?? null;
}

/** Profil membre par id métier (externalUserId ou id Better-Auth). */
export async function resolveMemberProfileById(userId: string): Promise<{
  id: string;
  firstName?: string;
  lastName?: string;
  email: string;
  imageUrl?: string;
  name: string;
} | null> {
  if (!isDatabaseConfigured() || !userId.trim()) return null;
  const db = getDb();
  const rows = await db.select().from(user);
  const hit = rows.find((u) => u.externalUserId === userId || u.id === userId);
  if (!hit) return null;
  return {
    id: hit.externalUserId ?? hit.id,
    firstName: hit.firstName ?? undefined,
    lastName: hit.lastName ?? undefined,
    email: hit.email,
    imageUrl: hit.image ?? undefined,
    name: hit.name,
  };
}
