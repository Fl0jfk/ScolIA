import "server-only";

import { eq } from "drizzle-orm";
import {
  setUserRolesInDb,
  syncUserAdminFlagsInDb,
  upsertAuthUserMapping,
} from "@/app/lib/auth-roles-db";
import type { DirectoryMemberRow } from "@/app/lib/directory-members";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { getTenant } from "@/app/lib/tenant-context";
import {
  ensureUserMembership,
  findUserIdByEmailNormalized,
} from "@/app/lib/user-membership";
import { isDatabaseConfigured, getDb } from "@/db/index";
import { user } from "@/db/schema";

export async function syncMemberRowToDatabase(member: DirectoryMemberRow): Promise<void> {
  if (!isDatabaseConfigured() || !member.externalUserId) return;
  const tenant = await getTenant();
  const etablissementId = await ensureEtablissementFromTenant(tenant);
  const db = getDb();

  const [existingByExt] = await db
    .select()
    .from(user)
    .where(eq(user.externalUserId, member.externalUserId))
    .limit(1);

  let userId = existingByExt?.id;

  // Rapprochement national : même e-mail → même compte, nouveau membership
  if (!userId && member.email?.trim()) {
    const byEmail = await findUserIdByEmailNormalized(member.email);
    if (byEmail) userId = byEmail;
  }

  if (!userId) {
    userId = crypto.randomUUID();
    await db.insert(user).values({
      id: userId,
      name: member.displayName ?? member.email,
      email: member.email,
      emailVerified: !member.pending,
      etablissementId,
      externalUserId: member.externalUserId,
      firstName: member.firstName,
      lastName: member.lastName,
      orgAdmin: member.roles.includes("admin"),
    });
  } else {
    await db
      .update(user)
      .set({
        email: member.email,
        name: member.displayName ?? member.email,
        firstName: member.firstName,
        lastName: member.lastName,
        // Ne pas écraser externalUserId d’un autre tenant si déjà posé
        ...(existingByExt ? {} : { externalUserId: member.externalUserId }),
        updatedAt: new Date(),
      })
      .where(eq(user.id, userId));
  }

  await ensureUserMembership({
    userId,
    etablissementId,
    context: "staff",
  });
  await upsertAuthUserMapping(etablissementId, member.externalUserId, userId);
  await setUserRolesInDb(userId, etablissementId, member.roles);
  await syncUserAdminFlagsInDb(userId, member.roles);
}

export function membersApiSourceLabel(): string {
  return isDatabaseConfigured() ? "better-auth" : "none";
}
