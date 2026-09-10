import "server-only";

import { and, eq, inArray, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { passkey } from "@/db/schema";

/**
 * true si l’utilisateur a au moins une passkey.
 * Ne jette jamais : une panne BDD ne doit pas déconnecter la session (proxy).
 */
export async function userHasPasskey(userId: string): Promise<boolean> {
  if (!userId || !isDatabaseConfigured()) return false;
  try {
    const db = getDb();
    const [row] = await db
      .select({ id: passkey.id })
      .from(passkey)
      .where(eq(passkey.userId, userId))
      .limit(1);
    return Boolean(row?.id);
  } catch (error) {
    console.error("[userHasPasskey]", error);
    return false;
  }
}

/**
 * Comme userHasPasskey, mais distingue « sûr que non » vs « contrôle impossible ».
 * Utilisé par le proxy pour ne pas renvoyer en setup-2fa si la BDD est saturée.
 */
export async function checkUserHasPasskey(
  userId: string,
): Promise<{ hasPasskey: boolean; checkFailed: boolean }> {
  if (!userId || !isDatabaseConfigured()) {
    return { hasPasskey: false, checkFailed: false };
  }
  try {
    const db = getDb();
    const [row] = await db
      .select({ id: passkey.id })
      .from(passkey)
      .where(eq(passkey.userId, userId))
      .limit(1);
    return { hasPasskey: Boolean(row?.id), checkFailed: false };
  } catch (error) {
    console.error("[checkUserHasPasskey]", error);
    return { hasPasskey: false, checkFailed: true };
  }
}

/** Compte de passkeys par userId (batch, pour listes membres). */
export async function countPasskeysByUserIds(
  userIds: string[],
): Promise<Map<string, number>> {
  const out = new Map<string, number>();
  if (userIds.length === 0 || !isDatabaseConfigured()) return out;
  try {
    const db = getDb();
    const rows = await db
      .select({
        userId: passkey.userId,
        n: sql<number>`count(*)::int`,
      })
      .from(passkey)
      .where(inArray(passkey.userId, userIds))
      .groupBy(passkey.userId);
    for (const r of rows) {
      out.set(r.userId, Number(r.n));
    }
  } catch (error) {
    console.error("[countPasskeysByUserIds]", error);
  }
  return out;
}

export async function listPasskeysForUser(userId: string): Promise<
  {
    id: string;
    name: string | null;
    deviceType: string;
    backedUp: boolean;
    createdAt: Date | null;
    aaguid: string | null;
  }[]
> {
  if (!userId || !isDatabaseConfigured()) return [];
  try {
    const db = getDb();
    return await db
      .select({
        id: passkey.id,
        name: passkey.name,
        deviceType: passkey.deviceType,
        backedUp: passkey.backedUp,
        createdAt: passkey.createdAt,
        aaguid: passkey.aaguid,
      })
      .from(passkey)
      .where(eq(passkey.userId, userId));
  } catch (error) {
    console.error("[listPasskeysForUser]", error);
    return [];
  }
}

export async function deletePasskeyForUser(
  userId: string,
  passkeyId: string,
): Promise<boolean> {
  if (!userId || !passkeyId || !isDatabaseConfigured()) return false;
  try {
    const db = getDb();
    const deleted = await db
      .delete(passkey)
      .where(and(eq(passkey.id, passkeyId), eq(passkey.userId, userId)))
      .returning({ id: passkey.id });
    return deleted.length > 0;
  } catch (error) {
    console.error("[deletePasskeyForUser]", error);
    return false;
  }
}
