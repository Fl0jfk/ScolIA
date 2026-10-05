import "server-only";

import { and, asc, desc, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  eleve,
  noteDevoir,
  noteMatiere,
  notePeriode,
  noteValeur,
} from "@/db/schema";

export type FamilleNoteRow = {
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  devoirId: string;
  devoirLibelle: string;
  dateDevoir: string | null;
  coefficient: string;
  matiereCode: string;
  matiereLibelle: string;
  periodeCode: string;
  periodeLibelle: string;
  periodeStatut: string;
  valeur: string | null;
  absent: boolean;
  dispense: boolean;
  appreciation: string | null;
};

/**
 * Notes visibles famille sans attendre la clôture de période.
 * Périodes clôturées restent aussi listées (historique en cours d’année).
 */
export async function listFamilleNotes(
  etablissementId: string,
  eleveIds: string[],
): Promise<FamilleNoteRow[]> {
  if (!eleveIds.length) return [];
  const db = getDb();

  const eleves = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), inArray(eleve.id, eleveIds)));
  const eleveById = new Map(eleves.map((e) => [e.id, e]));
  if (!eleves.length) return [];

  const rows = await db
    .select({
      eleveId: noteValeur.eleveId,
      devoirId: noteDevoir.id,
      devoirLibelle: noteDevoir.libelle,
      dateDevoir: noteDevoir.dateDevoir,
      coefficient: noteDevoir.coefficient,
      matiereCode: noteMatiere.code,
      matiereLibelle: noteMatiere.libelle,
      periodeCode: notePeriode.code,
      periodeLibelle: notePeriode.libelle,
      periodeStatut: notePeriode.statut,
      valeur: noteValeur.valeur,
      absent: noteValeur.absent,
      dispense: noteValeur.dispense,
      appreciation: noteValeur.appreciation,
    })
    .from(noteValeur)
    .innerJoin(noteDevoir, eq(noteValeur.devoirId, noteDevoir.id))
    .innerJoin(noteMatiere, eq(noteDevoir.matiereId, noteMatiere.id))
    .innerJoin(notePeriode, eq(noteDevoir.periodeId, notePeriode.id))
    .where(
      and(
        eq(noteValeur.etablissementId, etablissementId),
        eq(noteDevoir.etablissementId, etablissementId),
        inArray(noteValeur.eleveId, eleveIds),
      ),
    )
    .orderBy(desc(noteDevoir.dateDevoir), desc(noteDevoir.createdAt), asc(noteMatiere.code));

  const out: FamilleNoteRow[] = [];
  for (const r of rows) {
    const e = eleveById.get(r.eleveId);
    if (!e) continue;
    out.push({
      eleveId: r.eleveId,
      eleveNom: e.nom,
      elevePrenom: e.prenom,
      eleveClasse: e.classe,
      devoirId: r.devoirId,
      devoirLibelle: r.devoirLibelle,
      dateDevoir: r.dateDevoir,
      coefficient: String(r.coefficient ?? "1"),
      matiereCode: r.matiereCode,
      matiereLibelle: r.matiereLibelle,
      periodeCode: r.periodeCode,
      periodeLibelle: r.periodeLibelle,
      periodeStatut: r.periodeStatut,
      valeur: r.valeur != null ? String(r.valeur) : null,
      absent: r.absent,
      dispense: r.dispense,
      appreciation: r.appreciation,
    });
  }
  return out;
}
