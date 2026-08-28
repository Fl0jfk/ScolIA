import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { refNomenclature, noteMatiere } from "@/db/schema";
import { upsertMatiere } from "@/app/lib/notes-config-db";

/** Alimente note_matiere depuis ref_nomenclature (type matiere). */
export async function syncMatieresFromNomenclature(etablissementId: string): Promise<{
  inserts: number;
  updates: number;
}> {
  const db = getDb();
  const matieres = await db
    .select({
      code: refNomenclature.code,
      libelleLong: refNomenclature.libelleLong,
      libelleCourt: refNomenclature.libelleCourt,
      metadataJson: refNomenclature.metadataJson,
    })
    .from(refNomenclature)
    .where(
      and(eq(refNomenclature.etablissementId, etablissementId), eq(refNomenclature.type, "matiere")),
    );

  let inserts = 0;
  let updates = 0;

  for (const m of matieres) {
    const libelle = (m.libelleLong || m.libelleCourt || m.code).trim();
    const meta = m.metadataJson as { codeGestion?: string } | null;
    const groupeMatiere = meta?.codeGestion?.trim() || undefined;

    const existing = await db
      .select({ id: noteMatiere.id })
      .from(noteMatiere)
      .where(and(eq(noteMatiere.etablissementId, etablissementId), eq(noteMatiere.code, m.code)))
      .limit(1);

    await upsertMatiere(etablissementId, {
      code: m.code,
      libelle,
      groupeMatiere,
      actif: true,
    });

    if (existing[0]) updates += 1;
    else inserts += 1;
  }

  return { inserts, updates };
}
