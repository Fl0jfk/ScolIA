import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user } from "@/db/schema";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import {
  resolveMfaTrustPolicy,
  type MfaTrustPolicy,
} from "@/app/lib/two-factor-policy";

export async function resolveMfaTrustPolicyForUserId(
  userId: string,
): Promise<MfaTrustPolicy | null> {
  if (!isDatabaseConfigured() || !userId) return null;
  const db = getDb();
  const [row] = await db
    .select({
      id: user.id,
      etablissementId: user.etablissementId,
      platformAdmin: user.platformAdmin,
    })
    .from(user)
    .where(eq(user.id, userId))
    .limit(1);
  if (!row) return null;
  const roles = await listUserRolesFromDb(row.id, row.etablissementId);
  return resolveMfaTrustPolicy({
    platformAdmin: row.platformAdmin,
    roles,
  });
}

export async function resolveMfaTrustPolicyForEmail(
  email: string,
): Promise<MfaTrustPolicy | null> {
  if (!isDatabaseConfigured()) return null;
  const normalized = email.trim().toLowerCase();
  if (!normalized) return null;
  const db = getDb();
  const [row] = await db
    .select({
      id: user.id,
      etablissementId: user.etablissementId,
      platformAdmin: user.platformAdmin,
    })
    .from(user)
    .where(and(eq(user.email, normalized)))
    .limit(1);
  if (!row) return null;
  const roles = await listUserRolesFromDb(row.id, row.etablissementId);
  return resolveMfaTrustPolicy({
    platformAdmin: row.platformAdmin,
    roles,
  });
}
