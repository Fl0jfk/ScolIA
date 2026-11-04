import "server-only";

import { and, eq, or } from "drizzle-orm";
import { getDb } from "@/db/index";
import { user } from "@/db/schema";

/**
 * Écrit la date de naissance (AAAA-MM-JJ, déjà validée) sur le compte, restreint à l'établissement.
 * `userRef` peut être l'id du compte ou son external_user_id historique. Retourne true si une ligne a été mise à jour.
 */
export async function setUserBirthDate(
  etablissementId: string,
  userRef: string,
  birthDate: string,
): Promise<boolean> {
  const ref = userRef.trim();
  if (!ref) return false;
  const rows = await getDb()
    .update(user)
    .set({ dateNaissance: birthDate, updatedAt: new Date() })
    .where(and(eq(user.etablissementId, etablissementId), or(eq(user.id, ref), eq(user.externalUserId, ref))))
    .returning({ id: user.id });
  return rows.length > 0;
}
