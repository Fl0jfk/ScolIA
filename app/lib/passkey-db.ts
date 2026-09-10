import "server-only";

import { and, eq, inArray, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { passkey } from "@/db/schema";
import {
  cacheGetPasskeyPresence,
  cacheInvalidatePasskeyPresence,
  cacheSetPasskeyPresence,
} from "@/app/lib/valkey-cache";

/** Cache process court (L1) — Valkey en L2 partagé. */
const PASSKEY_CACHE_TTL_MS = 60_000;
const passkeyPresenceCache = new Map<
  string,
  { hasPasskey: boolean; expiresAt: number }
>();

export function invalidatePasskeyPresenceCache(userId?: string): void {
  if (!userId) {
    passkeyPresenceCache.clear();
    return;
  }
  passkeyPresenceCache.delete(userId);
  void cacheInvalidatePasskeyPresence(userId);
}

function readMemoryCache(userId: string): boolean | null {
  const hit = passkeyPresenceCache.get(userId);
  if (!hit) return null;
  if (hit.expiresAt <= Date.now()) {
    passkeyPresenceCache.delete(userId);
    return null;
  }
  return hit.hasPasskey;
}

function writeMemoryCache(userId: string, hasPasskey: boolean): void {
  passkeyPresenceCache.set(userId, {
    hasPasskey,
    expiresAt: Date.now() + PASSKEY_CACHE_TTL_MS,
  });
}

/**
 * true si l’utilisateur a au moins une passkey.
 * Ne jette jamais : une panne BDD ne doit pas déconnecter la session (proxy).
 */
export async function userHasPasskey(userId: string): Promise<boolean> {
  const checked = await checkUserHasPasskey(userId);
  return checked.hasPasskey;
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
  const mem = readMemoryCache(userId);
  if (mem !== null) {
    return { hasPasskey: mem, checkFailed: false };
  }
  const fromValkey = await cacheGetPasskeyPresence(userId);
  if (fromValkey !== null) {
    writeMemoryCache(userId, fromValkey);
    return { hasPasskey: fromValkey, checkFailed: false };
  }
  try {
    const db = getDb();
    const [row] = await db
      .select({ id: passkey.id })
      .from(passkey)
      .where(eq(passkey.userId, userId))
      .limit(1);
    const hasPasskey = Boolean(row?.id);
    writeMemoryCache(userId, hasPasskey);
    void cacheSetPasskeyPresence(userId, hasPasskey);
    return { hasPasskey, checkFailed: false };
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
      writeMemoryCache(r.userId, Number(r.n) > 0);
      void cacheSetPasskeyPresence(r.userId, Number(r.n) > 0);
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
    invalidatePasskeyPresenceCache(userId);
    return deleted.length > 0;
  } catch (error) {
    console.error("[deletePasskeyForUser]", error);
    return false;
  }
}
