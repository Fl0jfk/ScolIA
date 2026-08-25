import "server-only";

import { and, desc, eq, inArray } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, vsAbsenceEleve } from "@/db/schema";

export type FamilleAbsenceRow = {
  id: string;
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  dateDebut: string;
  dateFin: string;
  type: string;
  statut: string;
  justifie: boolean;
  motif: string | null;
  /** Motif déjà saisi par la famille, en attente de validation CPE. */
  motifEnAttente: boolean;
};

/** Absences des enfants — sans notes CPE internes. */
export async function listFamilleAbsences(
  etablissementId: string,
  eleveIds: string[],
): Promise<FamilleAbsenceRow[]> {
  if (!eleveIds.length) return [];
  const db = getDb();

  const rows = await db
    .select({
      id: vsAbsenceEleve.id,
      eleveId: vsAbsenceEleve.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      dateDebut: vsAbsenceEleve.dateDebut,
      dateFin: vsAbsenceEleve.dateFin,
      type: vsAbsenceEleve.type,
      statut: vsAbsenceEleve.statut,
      justifie: vsAbsenceEleve.justifie,
      motif: vsAbsenceEleve.motif,
    })
    .from(vsAbsenceEleve)
    .innerJoin(eleve, eq(eleve.id, vsAbsenceEleve.eleveId))
    .where(
      and(
        eq(vsAbsenceEleve.etablissementId, etablissementId),
        inArray(vsAbsenceEleve.eleveId, eleveIds),
      ),
    )
    .orderBy(desc(vsAbsenceEleve.dateDebut))
    .limit(100);

  return rows.map((r) => {
    const motifEnAttente = Boolean(r.motif?.trim()) && !r.justifie && r.statut === "a_traiter";
    return {
      ...r,
      motifEnAttente,
      statut: r.justifie
        ? "justifiee"
        : r.statut === "non_justifiee"
          ? "non_justifiee"
          : r.statut === "classee"
            ? "classee"
            : motifEnAttente
              ? "justif_recue"
              : "en_cours",
    };
  });
}

/**
 * Le parent dépose un motif de justification.
 * L’absence reste « à traiter » côté CPE jusqu’à validation staff.
 */
export async function submitFamilleAbsenceJustification(
  etablissementId: string,
  absenceId: string,
  eleveIdsAllowed: string[],
  motif: string,
): Promise<FamilleAbsenceRow | null> {
  const trimmed = motif.trim();
  if (trimmed.length < 3) {
    throw new Error("Indiquez un motif (au moins 3 caractères).");
  }
  if (trimmed.length > 500) {
    throw new Error("Motif trop long (500 caractères max).");
  }
  if (!eleveIdsAllowed.length) return null;

  const db = getDb();
  const [existing] = await db
    .select({
      id: vsAbsenceEleve.id,
      eleveId: vsAbsenceEleve.eleveId,
      statut: vsAbsenceEleve.statut,
      justifie: vsAbsenceEleve.justifie,
    })
    .from(vsAbsenceEleve)
    .where(
      and(eq(vsAbsenceEleve.etablissementId, etablissementId), eq(vsAbsenceEleve.id, absenceId)),
    )
    .limit(1);

  if (!existing || !eleveIdsAllowed.includes(existing.eleveId)) {
    return null;
  }
  if (existing.justifie || existing.statut === "classee" || existing.statut === "justifiee") {
    throw new Error("Cette absence est déjà traitée.");
  }
  if (existing.statut === "non_justifiee") {
    throw new Error("Cette absence a été classée non justifiée — contactez la vie scolaire.");
  }

  await db
    .update(vsAbsenceEleve)
    .set({
      motif: trimmed,
      statut: "a_traiter",
      updatedAt: new Date(),
    })
    .where(
      and(eq(vsAbsenceEleve.etablissementId, etablissementId), eq(vsAbsenceEleve.id, absenceId)),
    );

  const refreshed = await listFamilleAbsences(etablissementId, [existing.eleveId]);
  return refreshed.find((r) => r.id === absenceId) ?? null;
}
