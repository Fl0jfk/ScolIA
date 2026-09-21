import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { anneeScolaire } from "@/db/schema";
import {
  currentSchoolYearLabel,
  nextSchoolYearLabelFrom,
} from "@/app/lib/eleve-core/invariants";

export async function ensureAnneeScolaireByLabel(
  etablissementId: string,
  label: string,
): Promise<{ id: string; label: string }> {
  const db = getDb();
  const [existing] = await db
    .select({ id: anneeScolaire.id, label: anneeScolaire.label })
    .from(anneeScolaire)
    .where(and(eq(anneeScolaire.etablissementId, etablissementId), eq(anneeScolaire.label, label)))
    .limit(1);
  if (existing) return existing;

  const m = /^(\d{4})-(\d{4})$/.exec(label.trim());
  const startsOn = m ? `${m[1]}-09-01` : null;
  const endsOn = m ? `${m[2]}-08-31` : null;
  const [created] = await db
    .insert(anneeScolaire)
    .values({
      etablissementId,
      label,
      startsOn,
      endsOn,
      isCurrent: false,
    })
    .returning({ id: anneeScolaire.id, label: anneeScolaire.label });
  return created;
}

/** Même règle que `ensureCurrentAnneeScolaire` : le libellé calendaire devient l’année courante. */
export async function ensureCurrentAnneeId(etablissementId: string): Promise<string> {
  const db = getDb();
  const label = currentSchoolYearLabel();
  const row = await ensureAnneeScolaireByLabel(etablissementId, label);
  const [current] = await db
    .select({ id: anneeScolaire.id, isCurrent: anneeScolaire.isCurrent })
    .from(anneeScolaire)
    .where(eq(anneeScolaire.id, row.id))
    .limit(1);
  if (current && !current.isCurrent) {
    await db
      .update(anneeScolaire)
      .set({ isCurrent: false, updatedAt: new Date() })
      .where(eq(anneeScolaire.etablissementId, etablissementId));
    await db
      .update(anneeScolaire)
      .set({ isCurrent: true, updatedAt: new Date() })
      .where(eq(anneeScolaire.id, row.id));
  }
  return row.id;
}

export async function ensureNextAnneeId(etablissementId: string): Promise<string> {
  const nextLabel = nextSchoolYearLabelFrom(currentSchoolYearLabel());
  const row = await ensureAnneeScolaireByLabel(etablissementId, nextLabel);
  return row.id;
}
