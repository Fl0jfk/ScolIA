import "server-only";

import { and, asc, desc, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, vsSanction, vsSanctionType } from "@/db/schema";
import { sqlPersonNameMatches } from "@/app/lib/person-name-search";

const DEFAULT_TYPES: Array<{ code: string; libelle: string; gravite: number; ordre: number }> = [
  { code: "MOT_CARNET", libelle: "Observation / mot carnet", gravite: 1, ordre: 1 },
  { code: "AVERT", libelle: "Avertissement", gravite: 1, ordre: 2 },
  { code: "COLLE", libelle: "Heure de colle / retenue", gravite: 2, ordre: 3 },
  { code: "EXCL_COURS", libelle: "Exclusion de cours", gravite: 2, ordre: 4 },
  { code: "EXCL_TEMP", libelle: "Exclusion temporaire", gravite: 3, ordre: 5 },
  { code: "CONSEIL", libelle: "Conseil de discipline", gravite: 4, ordre: 6 },
  { code: "AVERT_CD", libelle: "Avertissement conseil de discipline", gravite: 3, ordre: 7 },
  { code: "BLAME", libelle: "Blâme", gravite: 4, ordre: 8 },
];

export async function ensureSanctionTypes(etablissementId: string) {
  const db = getDb();
  const existing = await db
    .select({ code: vsSanctionType.code })
    .from(vsSanctionType)
    .where(eq(vsSanctionType.etablissementId, etablissementId));
  const have = new Set(existing.map((r) => r.code));
  for (const t of DEFAULT_TYPES) {
    if (have.has(t.code)) continue;
    await db.insert(vsSanctionType).values({
      etablissementId,
      code: t.code,
      libelle: t.libelle,
      gravite: t.gravite,
      ordre: t.ordre,
      actif: true,
    });
  }
}

export async function listSanctionTypes(etablissementId: string) {
  await ensureSanctionTypes(etablissementId);
  const db = getDb();
  return db
    .select()
    .from(vsSanctionType)
    .where(and(eq(vsSanctionType.etablissementId, etablissementId), eq(vsSanctionType.actif, true)))
    .orderBy(asc(vsSanctionType.ordre), asc(vsSanctionType.libelle));
}

export async function listSanctions(
  etablissementId: string,
  opts?: { limit?: number; statut?: string; eleveId?: string },
) {
  const db = getDb();
  const limit = opts?.limit ?? 100;
  const statut = opts?.statut || "active";
  const clauses = [
    eq(vsSanction.etablissementId, etablissementId),
    eq(vsSanction.statut, statut),
  ];
  if (opts?.eleveId) {
    clauses.push(eq(vsSanction.eleveId, opts.eleveId));
  }

  return db
    .select({
      id: vsSanction.id,
      eleveId: vsSanction.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      typeId: vsSanction.typeId,
      typeCode: vsSanctionType.code,
      typeLibelle: vsSanctionType.libelle,
      gravite: vsSanctionType.gravite,
      dateSanction: vsSanction.dateSanction,
      motif: vsSanction.motif,
      statut: vsSanction.statut,
      createdByNom: vsSanction.createdByNom,
      createdAt: vsSanction.createdAt,
    })
    .from(vsSanction)
    .innerJoin(eleve, eq(eleve.id, vsSanction.eleveId))
    .innerJoin(vsSanctionType, eq(vsSanctionType.id, vsSanction.typeId))
    .where(and(...clauses))
    .orderBy(desc(vsSanction.dateSanction), desc(vsSanction.createdAt))
    .limit(limit);
}

export async function listSanctionsForEleve(
  etablissementId: string,
  eleveId: string,
  opts?: { limit?: number },
) {
  return listSanctions(etablissementId, {
    eleveId,
    statut: "active",
    limit: opts?.limit ?? 30,
  });
}

export async function createSanction(
  etablissementId: string,
  input: {
    eleveId: string;
    typeId: string;
    dateSanction: string;
    motif?: string | null;
    createdByUserId?: string | null;
    createdByNom?: string | null;
  },
) {
  const db = getDb();
  const eleveId = input.eleveId.trim();
  const typeId = input.typeId.trim();
  const dateSanction = input.dateSanction.trim();
  if (!eleveId || !typeId || !dateSanction) {
    throw new Error("Élève, type et date obligatoires.");
  }

  const [eleveRow] = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!eleveRow) throw new Error("Élève introuvable.");

  const [typeRow] = await db
    .select({ id: vsSanctionType.id })
    .from(vsSanctionType)
    .where(
      and(
        eq(vsSanctionType.etablissementId, etablissementId),
        eq(vsSanctionType.id, typeId),
        eq(vsSanctionType.actif, true),
      ),
    )
    .limit(1);
  if (!typeRow) throw new Error("Type de sanction introuvable.");

  const [row] = await db
    .insert(vsSanction)
    .values({
      etablissementId,
      eleveId,
      typeId,
      dateSanction,
      motif: input.motif?.trim() || null,
      statut: "active",
      createdByUserId: input.createdByUserId || null,
      createdByNom: input.createdByNom || null,
    })
    .returning();
  return row;
}

export async function annulerSanction(etablissementId: string, id: string) {
  const db = getDb();
  const [row] = await db
    .update(vsSanction)
    .set({ statut: "annulee", updatedAt: new Date() })
    .where(and(eq(vsSanction.etablissementId, etablissementId), eq(vsSanction.id, id)))
    .returning();
  return row ?? null;
}

export async function countSanctionsActives(etablissementId: string): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(vsSanction)
    .where(and(eq(vsSanction.etablissementId, etablissementId), eq(vsSanction.statut, "active")));
  return row?.n ?? 0;
}

/** Sanctions actives datées du jour (signal live direction / CPE). */
export async function countSanctionsAujourdhui(
  etablissementId: string,
  dateIso: string,
): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(vsSanction)
    .where(
      and(
        eq(vsSanction.etablissementId, etablissementId),
        eq(vsSanction.statut, "active"),
        eq(vsSanction.dateSanction, dateIso),
      ),
    );
  return row?.n ?? 0;
}

export async function searchElevesForSanction(
  etablissementId: string,
  q: string,
): Promise<Array<{ id: string; nom: string; prenom: string; classe: string | null }>> {
  const db = getDb();
  const needle = q.trim();
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
        sqlPersonNameMatches({
          nom: eleve.nom,
          prenom: eleve.prenom,
          extras: [eleve.classe],
          query: needle,
        }),
      ),
    )
    .orderBy(asc(eleve.nom), asc(eleve.prenom))
    .limit(20);
}
