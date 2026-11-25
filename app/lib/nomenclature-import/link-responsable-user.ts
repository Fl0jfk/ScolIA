import "server-only";

import { and, eq, isNull } from "drizzle-orm";
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

/**
 * Variante de l'import Excel des foyers : ne rattache que les responsables SANS compte
 * (`userId` nul) de l'établissement indiqué ; n'écrase jamais un rattachement existant.
 */
export async function linkFoyerResponsableToUserAccountIfUnlinked(
  etablissementId: string,
  responsableId: string,
  email: string | null | undefined,
): Promise<boolean> {
  const normalized = email?.trim().toLowerCase() || "";
  if (!normalized) return false;
  const userId = await findUserIdByEmailNormalized(normalized);
  if (!userId) return false;
  const db = getDb();
  const updated = await db
    .update(foyerResponsable)
    .set({ userId, updatedAt: new Date() })
    .where(
      and(
        eq(foyerResponsable.id, responsableId),
        eq(foyerResponsable.etablissementId, etablissementId),
        isNull(foyerResponsable.userId),
      ),
    )
    .returning({ id: foyerResponsable.id });
  if (!updated.length) return false;
  await ensureUserMembership({ userId, etablissementId, context: "parent" });
  return true;
}
