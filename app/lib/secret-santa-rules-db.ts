import "server-only";

import { and, eq, or, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { secretSantaIncompatibility } from "@/db/schema";
import { SANTA_MAX_RULES_PER_PERSON, SantaError, type SantaExclusion } from "@/app/lib/secret-santa";

/**
 * Incompatibilités confidentielles d'un établissement. Leur contenu ne sort que vers le tirage
 * et vers les routes réservées à l'administrateur.
 */

type Reader = Pick<ReturnType<typeof getDb>, "select">;

export async function listIncompatibilities(etabId: string, db: Reader = getDb()): Promise<SantaExclusion[]> {
  const rows = await db
    .select({ a: secretSantaIncompatibility.userA, b: secretSantaIncompatibility.userB })
    .from(secretSantaIncompatibility)
    .where(eq(secretSantaIncompatibility.etablissementId, etabId));
  return rows.map((r) => [r.a, r.b]);
}

const ordered = (a: string, b: string): SantaExclusion => (a < b ? [a, b] : [b, a]);

export async function addIncompatibility(etabId: string, a: string, b: string, createdBy: string): Promise<void> {
  if (a === b) throw new SantaError("Choisissez deux personnes différentes.");
  const [userA, userB] = ordered(a, b);
  await getDb().transaction(async (tx) => {
    // Deux ajouts simultanés ne doivent pas dépasser le plafond : un seul à la fois par établissement.
    await tx.execute(sql`select pg_advisory_xact_lock(hashtext(${`santa-rules:${etabId}`}))`);
    const rules = await listIncompatibilities(etabId, tx);
    if (rules.some(([x, y]) => x === userA && y === userB)) return;
    // Au-delà, aucun tirage ne pourrait respecter les règles sans devenir devinable.
    if ([userA, userB].some((id) => rules.filter((r) => r.includes(id)).length >= SANTA_MAX_RULES_PER_PERSON)) {
      throw new SantaError(
        `${SANTA_MAX_RULES_PER_PERSON} personnes à éviter au maximum pour chacun : au-delà, le tirage ne serait plus secret.`,
      );
    }
    await tx
      .insert(secretSantaIncompatibility)
      .values({ etablissementId: etabId, userA, userB, createdBy })
      .onConflictDoNothing();
  });
}

export async function removeIncompatibility(etabId: string, a: string, b: string): Promise<void> {
  await getDb()
    .delete(secretSantaIncompatibility)
    .where(
      and(
        eq(secretSantaIncompatibility.etablissementId, etabId),
        or(
          and(eq(secretSantaIncompatibility.userA, a), eq(secretSantaIncompatibility.userB, b)),
          and(eq(secretSantaIncompatibility.userA, b), eq(secretSantaIncompatibility.userB, a)),
        ),
      ),
    );
}
