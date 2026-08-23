import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { authUserMapping, user, userRole } from "@/db/schema";
import { hasMasterRole, normalizeIntranetRoles } from "@/app/lib/intranet-roles";

export async function listUserRolesFromDb(
  userId: string,
  etablissementId: string,
): Promise<string[]> {
  if (!isDatabaseConfigured()) return [];
  const db = getDb();
  const rows = await db
    .select({ role: userRole.role })
    .from(userRole)
    .where(and(eq(userRole.userId, userId), eq(userRole.etablissementId, etablissementId)));
  return normalizeIntranetRoles(rows.map((r) => r.role));
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
  if (normalized.length === 0) return;
  await db.insert(userRole).values(
    normalized.map((role) => ({
      userId,
      etablissementId,
      role,
    })),
  );
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
