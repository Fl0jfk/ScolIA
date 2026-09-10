import "server-only";

import { and, eq, inArray } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { authUserMapping, user, userRole } from "@/db/schema";
import { hasMasterRole, normalizeIntranetRoles } from "@/app/lib/intranet-roles";

const ROLES_TTL_MS = 60_000;
const rolesMem = new Map<string, { roles: string[]; expiresAt: number }>();

function rolesKey(userId: string, etablissementId: string): string {
  return `${userId}:${etablissementId}`;
}

export function invalidateUserRolesCache(
  userId?: string,
  etablissementId?: string,
): void {
  if (!userId) {
    rolesMem.clear();
    return;
  }
  if (etablissementId) {
    rolesMem.delete(rolesKey(userId, etablissementId));
    return;
  }
  for (const k of rolesMem.keys()) {
    if (k.startsWith(`${userId}:`)) rolesMem.delete(k);
  }
}

export async function listUserRolesFromDb(
  userId: string,
  etablissementId: string,
): Promise<string[]> {
  if (!isDatabaseConfigured() || !userId || !etablissementId) return [];

  const memKey = rolesKey(userId, etablissementId);
  const mem = rolesMem.get(memKey);
  if (mem && mem.expiresAt > Date.now()) return mem.roles;

  try {
    const { valkeyGetJson, valkeySetJson } = await import("@/app/lib/valkey");
    const vk = `scola:roles:${userId}:${etablissementId}`;
    const fromVk = await valkeyGetJson<string[]>(vk);
    if (fromVk) {
      rolesMem.set(memKey, { roles: fromVk, expiresAt: Date.now() + ROLES_TTL_MS });
      return fromVk;
    }

    const db = getDb();
    const rows = await db
      .select({ role: userRole.role })
      .from(userRole)
      .where(and(eq(userRole.userId, userId), eq(userRole.etablissementId, etablissementId)));
    const roles = normalizeIntranetRoles(rows.map((r) => r.role));
    rolesMem.set(memKey, { roles, expiresAt: Date.now() + ROLES_TTL_MS });
    void valkeySetJson(vk, roles, 60);
    return roles;
  } catch (error) {
    console.error("[listUserRolesFromDb]", error);
    // Repli direct Postgres sans cache si Valkey/import casse
    const db = getDb();
    const rows = await db
      .select({ role: userRole.role })
      .from(userRole)
      .where(and(eq(userRole.userId, userId), eq(userRole.etablissementId, etablissementId)));
    return normalizeIntranetRoles(rows.map((r) => r.role));
  }
}

/** Rôles pour plusieurs utilisateurs en une requête (évite N+1 sur l’annuaire). */
export async function listUserRolesBatchFromDb(
  userIds: string[],
  etablissementId: string,
): Promise<Map<string, string[]>> {
  const map = new Map<string, string[]>();
  if (!isDatabaseConfigured() || userIds.length === 0) return map;
  const db = getDb();
  const rows = await db
    .select({ userId: userRole.userId, role: userRole.role })
    .from(userRole)
    .where(and(eq(userRole.etablissementId, etablissementId), inArray(userRole.userId, userIds)));
  for (const row of rows) {
    const existing = map.get(row.userId) ?? [];
    existing.push(row.role);
    map.set(row.userId, existing);
  }
  for (const [userId, roles] of map) {
    map.set(userId, normalizeIntranetRoles(roles));
  }
  return map;
}

export async function setUserRolesInDb(
  userId: string,
  etablissementId: string,
  roles: string[],
): Promise<void> {
  if (!isDatabaseConfigured()) return;
  const db = getDb();
  const normalized = normalizeIntranetRoles(roles).filter((r) => r !== "master");
  await db
    .delete(userRole)
    .where(and(eq(userRole.userId, userId), eq(userRole.etablissementId, etablissementId)));
  if (normalized.length > 0) {
    await db.insert(userRole).values(
      normalized.map((role) => ({
        userId,
        etablissementId,
        role,
      })),
    );
  }
  invalidateUserRolesCache(userId, etablissementId);
  try {
    const { valkeyDel } = await import("@/app/lib/valkey");
    await valkeyDel(`scola:roles:${userId}:${etablissementId}`);
  } catch {
    /* ignore */
  }
}

export async function syncUserAdminFlagsInDb(
  userId: string,
  roles: string[],
  platformAdmin = false,
): Promise<void> {
  if (!isDatabaseConfigured()) return;
  const db = getDb();
  const orgAdmin = roles.includes("admin") || platformAdmin;
  await db
    .update(user)
    .set({ orgAdmin, platformAdmin, updatedAt: new Date() })
    .where(eq(user.id, userId));
}

export function isOrgAdminFromAppUser(input: {
  roles: string[];
  orgAdmin?: boolean;
  platformAdmin?: boolean;
}): boolean {
  if (input.platformAdmin) return true;
  if (input.orgAdmin) return true;
  return input.roles.includes("admin");
}

export function isPlatformMasterFromAppUser(input: {
  roles: string[];
  platformAdmin?: boolean;
}): boolean {
  if (input.platformAdmin) return true;
  return hasMasterRole(input.roles);
}

export async function resolveBusinessUserId(
  userId: string,
  etablissementId: string,
): Promise<string> {
  if (!isDatabaseConfigured()) return userId;
  const db = getDb();
  const [row] = await db
    .select({ externalUserId: user.externalUserId })
    .from(user)
    .where(and(eq(user.id, userId), eq(user.etablissementId, etablissementId)))
    .limit(1);
  return row?.externalUserId?.trim() || userId;
}

export async function upsertAuthUserMapping(
  etablissementId: string,
  externalUserId: string,
  userId: string,
): Promise<void> {
  if (!isDatabaseConfigured()) return;
  const db = getDb();
  await db
    .insert(authUserMapping)
    .values({ etablissementId, externalUserId, userId })
    .onConflictDoUpdate({
      target: [authUserMapping.etablissementId, authUserMapping.externalUserId],
      set: { userId, migratedAt: new Date() },
    });
  await db
    .update(user)
    .set({ externalUserId, updatedAt: new Date() })
    .where(eq(user.id, userId));
}

export async function findUserIdByExternalUserId(
  etablissementId: string,
  externalUserId: string,
): Promise<string | null> {
  if (!isDatabaseConfigured()) return null;
  const db = getDb();
  const [mapped] = await db
    .select({ userId: authUserMapping.userId })
    .from(authUserMapping)
    .where(
      and(
        eq(authUserMapping.etablissementId, etablissementId),
        eq(authUserMapping.externalUserId, externalUserId),
      ),
    )
    .limit(1);
  if (mapped) return mapped.userId;
  const [direct] = await db
    .select({ id: user.id })
    .from(user)
    .where(and(eq(user.etablissementId, etablissementId), eq(user.externalUserId, externalUserId)))
    .limit(1);
  return direct?.id ?? null;
}
