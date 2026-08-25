import "server-only";

import { and, asc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { noteMatiere, notePeriode, noteTypeDevoir, noteMatiereClasse } from "@/db/schema";

export async function listMatieres(etablissementId: string) {
  const db = getDb();
  return db
    .select()
    .from(noteMatiere)
    .where(eq(noteMatiere.etablissementId, etablissementId))
    .orderBy(asc(noteMatiere.code));
}

export async function upsertMatiere(
  etablissementId: string,
  input: { id?: string; code: string; libelle: string; groupeMatiere?: string; actif?: boolean },
) {
  const db = getDb();
  const code = input.code.trim().toUpperCase();
  const libelle = input.libelle.trim();
  if (!code || !libelle) throw new Error("Code et libellé obligatoires.");

  if (input.id) {
    const [row] = await db
      .update(noteMatiere)
      .set({
        code,
        libelle,
        groupeMatiere: input.groupeMatiere?.trim() || null,
        actif: input.actif !== false,
        updatedAt: new Date(),
      })
      .where(and(eq(noteMatiere.etablissementId, etablissementId), eq(noteMatiere.id, input.id)))
      .returning();
    return row;
  }

  const [row] = await db
    .insert(noteMatiere)
    .values({
      etablissementId,
      code,
      libelle,
      groupeMatiere: input.groupeMatiere?.trim() || null,
      actif: input.actif !== false,
    })
    .onConflictDoUpdate({
      target: [noteMatiere.etablissementId, noteMatiere.code],
      set: {
        libelle,
        groupeMatiere: input.groupeMatiere?.trim() || null,
        actif: input.actif !== false,
        updatedAt: new Date(),
      },
    })
    .returning();
  return row;
}

export async function listPeriodes(etablissementId: string) {
  const db = getDb();
  return db
    .select()
    .from(notePeriode)
    .where(eq(notePeriode.etablissementId, etablissementId))
    .orderBy(asc(notePeriode.ordre), asc(notePeriode.code));
}

export async function upsertPeriode(
  etablissementId: string,
  input: {
    id?: string;
    code: string;
    libelle: string;
    ordre?: number;
    niveauModele?: string;
    dateDebut?: string;
    dateFin?: string;
    statut?: string;
    anneeScolaireId?: string | null;
  },
) {
  const db = getDb();
  const code = input.code.trim().toUpperCase();
  const libelle = input.libelle.trim();
  if (!code || !libelle) throw new Error("Code et libellé obligatoires.");

  const values = {
    code,
    libelle,
    ordre: input.ordre ?? 1,
    niveauModele: input.niveauModele?.trim() || "tous",
    dateDebut: input.dateDebut || null,
    dateFin: input.dateFin || null,
    statut: input.statut?.trim() || "ouverte",
    anneeScolaireId: input.anneeScolaireId || null,
    updatedAt: new Date(),
  };

  if (!values.anneeScolaireId && !input.id) {
    const { resolveAnneeCouranteMeta } = await import("@/app/lib/annees-scolaires-db");
    values.anneeScolaireId = (await resolveAnneeCouranteMeta(etablissementId)).id;
  }

  if (input.id) {
    const [row] = await db
      .update(notePeriode)
      .set(values)
      .where(and(eq(notePeriode.etablissementId, etablissementId), eq(notePeriode.id, input.id)))
      .returning();
    return row;
  }

  const [row] = await db
    .insert(notePeriode)
    .values({ etablissementId, ...values })
    .returning();
  return row;
}

export async function listTypesDevoir(etablissementId: string) {
  const db = getDb();
  return db
    .select()
    .from(noteTypeDevoir)
    .where(eq(noteTypeDevoir.etablissementId, etablissementId))
    .orderBy(asc(noteTypeDevoir.code));
}

export async function seedNotesDefaults(etablissementId: string) {
  const existing = await listMatieres(etablissementId);
  if (existing.length > 0) return { seeded: false };

  const matieres = [
    { code: "MATHS", libelle: "Mathématiques" },
    { code: "FRAN", libelle: "Français" },
    { code: "HG", libelle: "Histoire-Géographie" },
    { code: "AGL1", libelle: "Anglais LV1" },
    { code: "EPS", libelle: "EPS" },
  ];
  for (const m of matieres) {
    await upsertMatiere(etablissementId, m);
  }

  const periodes = [
    { code: "T1", libelle: "1er trimestre", ordre: 1 },
    { code: "T2", libelle: "2e trimestre", ordre: 2 },
    { code: "T3", libelle: "3e trimestre", ordre: 3 },
  ];
  const { resolveAnneeCouranteMeta } = await import("@/app/lib/annees-scolaires-db");
  const annee = await resolveAnneeCouranteMeta(etablissementId);
  for (const p of periodes) {
    await upsertPeriode(etablissementId, { ...p, anneeScolaireId: annee.id });
  }

  const db = getDb();
  const types = [
    { code: "DS", libelle: "Devoir surveillé" },
    { code: "DM", libelle: "Devoir maison" },
    { code: "INT", libelle: "Interrogation" },
    { code: "ORAL", libelle: "Oral" },
  ];
  for (const t of types) {
    await db
      .insert(noteTypeDevoir)
      .values({
        etablissementId,
        code: t.code,
        libelle: t.libelle,
      })
      .onConflictDoNothing();
  }

  return { seeded: true };
}

export async function listMatieresClasse(etablissementId: string, classe?: string) {
  const db = getDb();
  const rows = await db
    .select()
    .from(noteMatiereClasse)
    .where(eq(noteMatiereClasse.etablissementId, etablissementId));
  if (!classe?.trim()) return rows;
  const c = classe.trim().toUpperCase();
  return rows.filter((r) => r.classe.trim().toUpperCase() === c);
}
