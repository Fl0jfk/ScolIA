import "server-only";

import { and, asc, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  anneeScolaire,
  eleve,
  noteMatiere,
  noteMatiereClasse,
  noteMoyenneEleve,
  notePeriode,
} from "@/db/schema";
import { COMPETENCE_NIVEAUX, listCompetencesForEleve } from "@/app/lib/notes-competences-db";

export type BulletinLigne = {
  matiereId: string;
  matiereCode: string;
  matiereLibelle: string;
  coef: string;
  compteDansMg: boolean;
  moyenne: string | null;
  nbNotes: number;
  enseignantNom: string | null;
};

export type BulletinSnapshot = {
  eleve: {
    id: string;
    nom: string;
    prenom: string;
    classe: string | null;
    ine: string | null;
  };
  periode: {
    id: string;
    code: string;
    libelle: string;
    statut: string;
  };
  anneeLabel: string;
  lignes: BulletinLigne[];
  moyenneGenerale: string | null;
  competences: Array<{
    domaineLibelle: string;
    itemLibelle: string;
    niveau: string | null;
    niveauLabel: string;
  }>;
};

function computeMoyenneGenerale(lignes: BulletinLigne[]): string | null {
  let sum = 0;
  let totalCoef = 0;
  for (const l of lignes) {
    if (!l.compteDansMg || l.moyenne == null) continue;
    const m = Number(l.moyenne);
    const c = Number(l.coef) || 1;
    if (!Number.isFinite(m)) continue;
    sum += m * c;
    totalCoef += c;
  }
  if (totalCoef <= 0) return null;
  return (sum / totalCoef).toFixed(2);
}

export async function loadBulletinSnapshot(
  etablissementId: string,
  eleveId: string,
  periodeId: string,
): Promise<BulletinSnapshot | null> {
  const db = getDb();

  const [eleveRow] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      ine: eleve.ine,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);

  if (!eleveRow) return null;

  const [periodeRow] = await db
    .select({
      id: notePeriode.id,
      code: notePeriode.code,
      libelle: notePeriode.libelle,
      statut: notePeriode.statut,
      anneeScolaireId: notePeriode.anneeScolaireId,
    })
    .from(notePeriode)
    .where(and(eq(notePeriode.etablissementId, etablissementId), eq(notePeriode.id, periodeId)))
    .limit(1);

  if (!periodeRow) return null;

  let anneeLabel = "Année scolaire en cours";
  if (periodeRow.anneeScolaireId) {
    const [annee] = await db
      .select({ label: anneeScolaire.label })
      .from(anneeScolaire)
      .where(
        and(
          eq(anneeScolaire.etablissementId, etablissementId),
          eq(anneeScolaire.id, periodeRow.anneeScolaireId),
        ),
      )
      .limit(1);
    if (annee?.label) anneeLabel = annee.label;
  }

  const classe = eleveRow.classe?.trim() || "";

  const matiereClasseJoin = and(
    eq(noteMatiereClasse.matiereId, noteMoyenneEleve.matiereId),
    eq(noteMatiereClasse.etablissementId, etablissementId),
    ...(classe ? [eq(noteMatiereClasse.classe, classe)] : []),
  );

  const moyennes = await db
    .select({
      matiereId: noteMoyenneEleve.matiereId,
      matiereCode: noteMatiere.code,
      matiereLibelle: noteMatiere.libelle,
      moyenne: noteMoyenneEleve.moyenne,
      nbNotes: noteMoyenneEleve.nbNotes,
      coef: noteMatiereClasse.coef,
      compteDansMg: noteMatiereClasse.compteDansMg,
      enseignantNom: noteMatiereClasse.enseignantNom,
    })
    .from(noteMoyenneEleve)
    .innerJoin(noteMatiere, eq(noteMoyenneEleve.matiereId, noteMatiere.id))
    .leftJoin(noteMatiereClasse, matiereClasseJoin)
    .where(
      and(
        eq(noteMoyenneEleve.etablissementId, etablissementId),
        eq(noteMoyenneEleve.eleveId, eleveId),
        eq(noteMoyenneEleve.periodeId, periodeId),
      ),
    )
    .orderBy(asc(noteMatiere.code));

  const lignes: BulletinLigne[] = moyennes.map((r) => ({
    matiereId: r.matiereId,
    matiereCode: r.matiereCode,
    matiereLibelle: r.matiereLibelle,
    coef: r.coef != null ? String(r.coef) : "1",
    compteDansMg: r.compteDansMg ?? true,
    moyenne: r.moyenne != null ? String(r.moyenne) : null,
    nbNotes: r.nbNotes,
    enseignantNom: r.enseignantNom,
  }));

  let competences: BulletinSnapshot["competences"] = [];
  try {
    const rows = await listCompetencesForEleve(etablissementId, eleveId, periodeId);
    competences = rows
      .filter((r) => r.niveau)
      .map((r) => ({
        domaineLibelle: r.domaineLibelle,
        itemLibelle: r.itemLibelle,
        niveau: r.niveau,
        niveauLabel: COMPETENCE_NIVEAUX.find((n) => n.code === r.niveau)?.label || "—",
      }));
  } catch {
    competences = [];
  }

  return {
    eleve: eleveRow,
    periode: {
      id: periodeRow.id,
      code: periodeRow.code,
      libelle: periodeRow.libelle,
      statut: periodeRow.statut,
    },
    anneeLabel,
    lignes,
    moyenneGenerale: computeMoyenneGenerale(lignes),
    competences,
  };
}

export async function listEleveIdsForBulletinClasse(
  etablissementId: string,
  classe: string,
): Promise<Array<{ id: string; nom: string; prenom: string }>> {
  const db = getDb();
  const trimmed = classe.trim();
  return db
    .select({ id: eleve.id, nom: eleve.nom, prenom: eleve.prenom })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        sql`lower(trim(${eleve.classe})) = lower(${trimmed})`,
      ),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom));
}

export async function listEleveIdsForBulletinGroupe(
  etablissementId: string,
  groupeId: string,
): Promise<Array<{ id: string; nom: string; prenom: string }>> {
  const { listElevesForGroupe } = await import("@/app/lib/notes-saisie-db");
  const rows = await listElevesForGroupe(etablissementId, groupeId);
  return rows.map((r) => ({ id: r.id, nom: r.nom, prenom: r.prenom }));
}
