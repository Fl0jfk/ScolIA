import "server-only";

import { eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { twoFactor, user } from "@/db/schema";

let columnHealthy = false;

/**
 * Alignement colonne `verified` : défaut false + nettoyage des setups MFA incomplets.
 * (Better-Auth / ancienne migration mettaient DEFAULT true, ce qui empêche la promotion
 * de `user.two_factor_enabled` après verifyTotp.)
 */
export async function ensureTwoFactorVerifiedColumnHealthy(): Promise<void> {
  if (columnHealthy) return;
  const db = getDb();
  await db.execute(
    sql.raw(`ALTER TABLE "two_factor" ALTER COLUMN "verified" SET DEFAULT false`),
  );
  await db.execute(sql.raw(`
    DELETE FROM "two_factor" AS tf
    USING "user" AS u
    WHERE tf."user_id" = u."id"
      AND u."two_factor_enabled" = false
  `));
  await db.execute(sql.raw(`
    UPDATE "two_factor" AS tf
    SET "verified" = true
    FROM "user" AS u
    WHERE tf."user_id" = u."id"
      AND u."two_factor_enabled" = true
      AND tf."verified" IS DISTINCT FROM true
  `));
  columnHealthy = true;
}

/**
 * Supprime un secret TOTP / codes de secours si la MFA n’est pas encore activée
 * sur le compte (abandon mid-QR, double génération, état `verified` incohérent).
 */
export async function clearIncompleteTwoFactorSetup(userId: string): Promise<boolean> {
  await ensureTwoFactorVerifiedColumnHealthy();
  const db = getDb();
  const [row] = await db
    .select({ twoFactorEnabled: user.twoFactorEnabled })
    .from(user)
    .where(eq(user.id, userId))
    .limit(1);
  if (!row || row.twoFactorEnabled) return false;
  await db.delete(twoFactor).where(eq(twoFactor.userId, userId));
  return true;
}

/**
 * Filet après verifyTotp : Better-Auth n’active `twoFactorEnabled` que si
 * `two_factor.verified !== true`. Si `verified` est déjà true (défaut SQL),
 * le code est accepté mais le flag user reste false → boucle setup-2fa.
 */
export async function forcePromoteTwoFactorEnabled(userId: string): Promise<boolean> {
  await ensureTwoFactorVerifiedColumnHealthy();
  const db = getDb();
  const [row] = await db
    .select({ twoFactorEnabled: user.twoFactorEnabled })
    .from(user)
    .where(eq(user.id, userId))
    .limit(1);
  if (!row) return false;

  const secrets = await db
    .select({ id: twoFactor.id })
    .from(twoFactor)
    .where(eq(twoFactor.userId, userId))
    .limit(1);
  if (secrets.length === 0) return false;

  const now = new Date();
  if (!row.twoFactorEnabled) {
    await db
      .update(user)
      .set({ twoFactorEnabled: true, updatedAt: now })
      .where(eq(user.id, userId));
  }
  await db
    .update(twoFactor)
    .set({ verified: true })
    .where(eq(twoFactor.userId, userId));
  return true;
}
