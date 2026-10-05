import "server-only";

import { and, asc, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { edtCreneau, noteMatiere } from "@/db/schema";

export type FamilleEdtCreneau = {
  id: string;
  jourSemaine: number;
  heureDebut: string;
  heureFin: string;
  classe: string | null;
  matiereLibelle: string | null;
  enseignantNom: string | null;
  salle: string | null;
  semaine: string;
};

export type FamilleEdtEnfantBlock = {
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  classe: string | null;
  creneaux: FamilleEdtCreneau[];
};

/**
 * Emploi du temps publié (`edt_creneau`) pour une classe.
 * Matching insensible à la casse / espaces.
 */
export async function listFamilleEdtForClasse(
  etablissementId: string,
  classe: string,
): Promise<FamilleEdtCreneau[]> {
  const cls = classe.trim();
  if (!cls) return [];
  const db = getDb();

  const rows = await db
    .select({
      id: edtCreneau.id,
      jourSemaine: edtCreneau.jourSemaine,
      heureDebut: edtCreneau.heureDebut,
      heureFin: edtCreneau.heureFin,
      classe: edtCreneau.classe,
      matiereLibelle: noteMatiere.libelle,
      enseignantNom: edtCreneau.enseignantNom,
      salle: edtCreneau.salle,
      semaine: edtCreneau.semaine,
    })
    .from(edtCreneau)
    .leftJoin(noteMatiere, eq(noteMatiere.id, edtCreneau.matiereId))
    .where(
      and(
        eq(edtCreneau.etablissementId, etablissementId),
        sql`lower(trim(${edtCreneau.classe})) = lower(${cls})`,
      ),
    )
    .orderBy(asc(edtCreneau.jourSemaine), asc(edtCreneau.heureDebut));

  return rows.map((r) => ({
    id: r.id,
    jourSemaine: r.jourSemaine,
    heureDebut: r.heureDebut,
    heureFin: r.heureFin,
    classe: r.classe,
    matiereLibelle: r.matiereLibelle,
    enseignantNom: r.enseignantNom,
    salle: r.salle,
    semaine: r.semaine,
  }));
}

/** Une entrée par enfant lié (créneaux = classe de l’élève). */
export async function listFamilleEdtByEnfants(
  etablissementId: string,
  enfants: Array<{ id: string; nom: string; prenom: string; classe: string | null }>,
): Promise<FamilleEdtEnfantBlock[]> {
  const classes = [
    ...new Set(
      enfants
        .map((e) => e.classe?.trim())
        .filter((c): c is string => Boolean(c)),
    ),
  ];
  const byClasse = new Map<string, FamilleEdtCreneau[]>();
  await Promise.all(
    classes.map(async (c) => {
      byClasse.set(c.toLowerCase(), await listFamilleEdtForClasse(etablissementId, c));
    }),
  );

  return enfants.map((e) => {
    const key = e.classe?.trim().toLowerCase() || "";
    return {
      eleveId: e.id,
      eleveNom: e.nom,
      elevePrenom: e.prenom,
      classe: e.classe,
      creneaux: key ? byClasse.get(key) || [] : [],
    };
  });
}
