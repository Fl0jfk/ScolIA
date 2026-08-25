import "server-only";

import { and, asc, desc, eq, inArray, isNull, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, vsCarnetEntree } from "@/db/schema";

export const CARNET_CATEGORIES = [
  { id: "correspondance", label: "Correspondance" },
  { id: "accompagnement", label: "Accompagnement pédagogique" },
  { id: "information", label: "Information" },
] as const;

export type CarnetCategorie = (typeof CARNET_CATEGORIES)[number]["id"];

function isCarnetCategorie(raw: string): raw is CarnetCategorie {
  return CARNET_CATEGORIES.some((c) => c.id === raw);
}

export async function listCarnetEntrees(
  etablissementId: string,
  opts?: {
    limit?: number;
    eleveId?: string;
    categorie?: string;
    nonSignees?: boolean;
  },
) {
  const db = getDb();
  const limit = opts?.limit ?? 100;
  const clauses = [eq(vsCarnetEntree.etablissementId, etablissementId)];
  if (opts?.eleveId) clauses.push(eq(vsCarnetEntree.eleveId, opts.eleveId));
  if (opts?.categorie && isCarnetCategorie(opts.categorie)) {
    clauses.push(eq(vsCarnetEntree.categorie, opts.categorie));
  }
  if (opts?.nonSignees) clauses.push(isNull(vsCarnetEntree.signeAt));

  return db
    .select({
      id: vsCarnetEntree.id,
      eleveId: vsCarnetEntree.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      dateEntree: vsCarnetEntree.dateEntree,
      categorie: vsCarnetEntree.categorie,
      titre: vsCarnetEntree.titre,
      corps: vsCarnetEntree.corps,
      visibleFamille: vsCarnetEntree.visibleFamille,
      createdByNom: vsCarnetEntree.createdByNom,
      signeAt: vsCarnetEntree.signeAt,
      signeParNom: vsCarnetEntree.signeParNom,
      createdAt: vsCarnetEntree.createdAt,
    })
    .from(vsCarnetEntree)
    .innerJoin(eleve, eq(eleve.id, vsCarnetEntree.eleveId))
    .where(and(...clauses))
    .orderBy(desc(vsCarnetEntree.dateEntree), desc(vsCarnetEntree.createdAt))
    .limit(limit);
}

export async function listCarnetForEleve(
  etablissementId: string,
  eleveId: string,
  opts?: { limit?: number },
) {
  return listCarnetEntrees(etablissementId, {
    eleveId,
    limit: opts?.limit ?? 40,
  });
}

export async function listFamilleCarnet(
  etablissementId: string,
  eleveIds: string[],
): Promise<
  Array<{
    id: string;
    eleveId: string;
    eleveNom: string;
    elevePrenom: string;
    eleveClasse: string | null;
    dateEntree: string;
    categorie: string;
    titre: string;
    corps: string;
    signeAt: Date | null;
    signeParNom: string | null;
    createdByNom: string | null;
  }>
> {
  if (!eleveIds.length) return [];
  const db = getDb();
  return db
    .select({
      id: vsCarnetEntree.id,
      eleveId: vsCarnetEntree.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      dateEntree: vsCarnetEntree.dateEntree,
      categorie: vsCarnetEntree.categorie,
      titre: vsCarnetEntree.titre,
      corps: vsCarnetEntree.corps,
      signeAt: vsCarnetEntree.signeAt,
      signeParNom: vsCarnetEntree.signeParNom,
      createdByNom: vsCarnetEntree.createdByNom,
    })
    .from(vsCarnetEntree)
    .innerJoin(eleve, eq(eleve.id, vsCarnetEntree.eleveId))
    .where(
      and(
        eq(vsCarnetEntree.etablissementId, etablissementId),
        inArray(vsCarnetEntree.eleveId, eleveIds),
        eq(vsCarnetEntree.visibleFamille, true),
      ),
    )
    .orderBy(desc(vsCarnetEntree.dateEntree), desc(vsCarnetEntree.createdAt))
    .limit(100);
}

export async function createCarnetEntree(
  etablissementId: string,
  input: {
    eleveId: string;
    dateEntree: string;
    categorie: string;
    titre: string;
    corps: string;
    visibleFamille?: boolean;
    createdByUserId?: string | null;
    createdByNom?: string | null;
  },
) {
  const db = getDb();
  const eleveId = input.eleveId.trim();
  const dateEntree = input.dateEntree.trim();
  const titre = input.titre.trim();
  const corps = input.corps.trim();
  const categorie = isCarnetCategorie(input.categorie) ? input.categorie : "correspondance";
  if (!eleveId || !dateEntree || !titre || !corps) {
    throw new Error("Élève, date, titre et message obligatoires.");
  }

  const [eleveRow] = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!eleveRow) throw new Error("Élève introuvable.");

  const [row] = await db
    .insert(vsCarnetEntree)
    .values({
      etablissementId,
      eleveId,
      dateEntree,
      categorie,
      titre,
      corps,
      visibleFamille: input.visibleFamille !== false,
      createdByUserId: input.createdByUserId || null,
      createdByNom: input.createdByNom || null,
    })
    .returning();
  return row;
}

export async function signerCarnetEntree(
  etablissementId: string,
  id: string,
  opts: {
    eleveIds: string[];
    userId: string;
    userNom: string | null;
  },
) {
  if (!opts.eleveIds.length) throw new Error("Aucun enfant rattaché.");
  const db = getDb();
  const [existing] = await db
    .select({
      id: vsCarnetEntree.id,
      eleveId: vsCarnetEntree.eleveId,
      visibleFamille: vsCarnetEntree.visibleFamille,
      signeAt: vsCarnetEntree.signeAt,
    })
    .from(vsCarnetEntree)
    .where(and(eq(vsCarnetEntree.etablissementId, etablissementId), eq(vsCarnetEntree.id, id)))
    .limit(1);
  if (!existing) throw new Error("Entrée introuvable.");
  if (!existing.visibleFamille) throw new Error("Entrée non visible famille.");
  if (!opts.eleveIds.includes(existing.eleveId)) {
    throw new Error("Accès refusé à cet élève.");
  }
  if (existing.signeAt) return existing;

  const [row] = await db
    .update(vsCarnetEntree)
    .set({
      signeAt: new Date(),
      signeParUserId: opts.userId,
      signeParNom: opts.userNom,
      updatedAt: new Date(),
    })
    .where(and(eq(vsCarnetEntree.etablissementId, etablissementId), eq(vsCarnetEntree.id, id)))
    .returning();
  return row ?? null;
}

export async function countCarnetNonSignees(etablissementId: string): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(vsCarnetEntree)
    .where(
      and(
        eq(vsCarnetEntree.etablissementId, etablissementId),
        eq(vsCarnetEntree.visibleFamille, true),
        isNull(vsCarnetEntree.signeAt),
      ),
    );
  return row?.n ?? 0;
}

export async function searchElevesForCarnet(
  etablissementId: string,
  q: string,
): Promise<Array<{ id: string; nom: string; prenom: string; classe: string | null }>> {
  const db = getDb();
  const needle = q.trim().toLowerCase();
  if (needle.length < 2) return [];
  return db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        eq(eleve.status, "inscrit"),
        sql`(lower(${eleve.nom}) like ${`%${needle}%`} or lower(${eleve.prenom}) like ${`%${needle}%`} or lower(coalesce(${eleve.classe}, '')) like ${`%${needle}%`})`,
      ),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom))
    .limit(20);
}
