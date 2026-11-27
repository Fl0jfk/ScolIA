import "server-only";

import { and, eq, inArray, isNull, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { personnel, user } from "@/db/schema";
import { secretSanta, secretSantaParticipant } from "@/db/schema-secret-santa";
import type { MemberName } from "@/app/lib/member-name";

/**
 * Renomme un compte et les copies « vivantes » de son nom : dossier du personnel de l'établissement
 * et Secret Santa. L'historique (réservations, demandes, documents) garde le nom de l'époque.
 */
export async function renameMember(
  etablissementId: string,
  account: { id: string; externalUserId: string | null; email: string },
  next: MemberName,
): Promise<Date> {
  const now = new Date();
  const { firstName, lastName, displayName } = next;
  await getDb().transaction(async (tx) => {
    await tx
      .update(user)
      .set({ firstName, lastName, name: displayName, updatedAt: now })
      .where(and(eq(user.id, account.id), eq(user.etablissementId, etablissementId)));

    const ids = [account.id, account.externalUserId].filter((id): id is string => Boolean(id));
    // Le rapprochement par e-mail ne vaut que pour un dossier qui n'est lié à aucun compte.
    const unlinkedSameEmail = and(
      or(isNull(personnel.externalUserId), eq(personnel.externalUserId, "")),
      sql`lower(${personnel.email}) = ${account.email.trim().toLowerCase()}`,
    );
    await tx
      .update(personnel)
      .set({ firstName, lastName, displayName, updatedAt: now })
      .where(
        and(
          eq(personnel.etablissementId, etablissementId),
          or(inArray(personnel.externalUserId, ids), unlinkedSameEmail),
        ),
      );

    await tx
      .update(secretSantaParticipant)
      .set({ displayName })
      .where(eq(secretSantaParticipant.userId, account.id));
    await tx
      .update(secretSanta)
      .set({ organizerName: displayName })
      .where(eq(secretSanta.organizerUserId, account.id));
  });
  return now;
}
