import "server-only";

import { eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { foyerResponsable } from "@/db/schema";
import { ensureUserMembership, findUserIdByEmailNormalized } from "@/app/lib/user-membership";

/** Rapproche un responsable légal Siècle avec un compte ScolIA existant (e-mail). */
export async function linkFoyerResponsableToUserAccount(
  etablissementId: string,
  responsableId: string,
  email: string | null | undefined,
): Promise<boolean> {
  const normalized = email?.trim().toLowerCase() || "";
  if (!normalized) return false;

  const userId = await findUserIdByEmailNormalized(normalized);
  if (!userId) return false;

  const db = getDb();
  await db
    .update(foyerResponsable)
    .set({ userId, updatedAt: new Date() })
    .where(eq(foyerResponsable.id, responsableId));

  await ensureUserMembership({
    userId,
    etablissementId,
    context: "parent",
  });

  return true;
}
