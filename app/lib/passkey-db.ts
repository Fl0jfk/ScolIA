import "server-only";

import { and, count, eq, inArray } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { passkey } from "@/db/schema";

/** true si l’utilisateur a au moins une passkey enregistrée. */
export async function userHasPasskey(userId: string): Promise<boolean> {
  if (!userId || !isDatabaseConfigured()) return false;
  const db = getDb();
  const [row] = await db
    .select({ n: count() })
    .from(passkey)
    .where(eq(passkey.userId, userId))
    .limit(1);
  return Number(row?.n ?? 0) > 0;
}

/** Compte de passkeys par userId (batch, pour listes membres). */
export async function countPasskeysByUserIds(
  userIds: string[],
): Promise<Map<string, number>> {
  const out = new Map<string, number>();
  if (userIds.length === 0 || !isDatabaseConfigured()) return out;
  const db = getDb();
  const rows = await db
    .select({ userId: passkey.userId, n: count() })
    .from(passkey)
    .where(inArray(passkey.userId, userIds))
    .groupBy(passkey.userId);
  for (const r of rows) {
    out.set(r.userId, Number(r.n));
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
  const db = getDb();
  return db
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
}

export async function deletePasskeyForUser(
  userId: string,
  passkeyId: string,
): Promise<boolean> {
  if (!userId || !passkeyId || !isDatabaseConfigured()) return false;
  const db = getDb();
  const deleted = await db
    .delete(passkey)
    .where(and(eq(passkey.id, passkeyId), eq(passkey.userId, userId)))
    .returning({ id: passkey.id });
  return deleted.length > 0;
}
