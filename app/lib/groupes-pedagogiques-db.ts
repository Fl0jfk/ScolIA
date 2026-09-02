import "server-only";

import { and, asc, count, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, groupePedagogique, groupePedagogiqueMembre } from "@/db/schema";
import { sqlPersonNameMatches } from "@/app/lib/person-name-search";

export type GroupeType = "option" | "lv2" | "sport" | "internat" | "autre";

export type GroupeInput = {
  id?: string;
  code: string;
  libelle: string;
  type?: GroupeType | string;
  anneeScolaireId?: string | null;
};

export type GroupeWithCount = {
  id: string;
  code: string;
  libelle: string;
  type: string;
  anneeScolaireId: string | null;
  memberCount: number;
};

export type GroupeMembreRow = {
  eleveId: string;
  nom: string;
  prenom: string;
  classe: string | null;
  ine: string | null;
};

export type EleveGroupeRow = {
  id: string;
  code: string;
  libelle: string;
  type: string;
};

export async function listGroupesForEleve(
  etablissementId: string,
  eleveId: string,
): Promise<EleveGroupeRow[]> {
  const db = getDb();
  const rows = await db
    .select({
      id: groupePedagogique.id,
      code: groupePedagogique.code,
      libelle: groupePedagogique.libelle,
      type: groupePedagogique.type,
    })
    .from(groupePedagogiqueMembre)
    .innerJoin(groupePedagogique, eq(groupePedagogiqueMembre.groupeId, groupePedagogique.id))
    .where(
      and(
        eq(groupePedagogiqueMembre.etablissementId, etablissementId),
        eq(groupePedagogiqueMembre.eleveId, eleveId),
      ),
    )
    .orderBy(asc(groupePedagogique.code));

  return rows;
}

export async function listGroupes(etablissementId: string): Promise<GroupeWithCount[]> {
  const db = getDb();
  const groupes = await db
    .select()
    .from(groupePedagogique)
    .where(eq(groupePedagogique.etablissementId, etablissementId))
    .orderBy(asc(groupePedagogique.code));

  if (!groupes.length) return [];

  const counts = await db
    .select({
      groupeId: groupePedagogiqueMembre.groupeId,
      n: count(),
    })
    .from(groupePedagogiqueMembre)
    .where(eq(groupePedagogiqueMembre.etablissementId, etablissementId))
    .groupBy(groupePedagogiqueMembre.groupeId);

  const countByGroupe = new Map(counts.map((c) => [c.groupeId, Number(c.n)]));

  return groupes.map((g) => ({
    id: g.id,
    code: g.code,
    libelle: g.libelle,
    type: g.type,
    anneeScolaireId: g.anneeScolaireId,
    memberCount: countByGroupe.get(g.id) ?? 0,
  }));
}

export async function listGroupeMembres(
  etablissementId: string,
  groupeId: string,
): Promise<GroupeMembreRow[]> {
  const db = getDb();
  const links = await db
    .select({ eleveId: groupePedagogiqueMembre.eleveId })
    .from(groupePedagogiqueMembre)
    .where(
      and(
        eq(groupePedagogiqueMembre.etablissementId, etablissementId),
        eq(groupePedagogiqueMembre.groupeId, groupeId),
      ),
    );

  if (!links.length) return [];

  const ids = links.map((l) => l.eleveId);
  const rows = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      ine: eleve.ine,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), inArray(eleve.id, ids)));

  const byId = new Map(rows.map((r) => [r.id, r]));
  return links
    .map((l) => byId.get(l.eleveId))
    .filter((r): r is NonNullable<typeof r> => Boolean(r))
    .map((r) => ({
      eleveId: r.id,
      nom: r.nom,
      prenom: r.prenom,
      classe: r.classe,
      ine: r.ine,
    }));
}

export async function upsertGroupe(etablissementId: string, input: GroupeInput) {
  const db = getDb();
  const code = input.code.trim().toUpperCase();
  const libelle = input.libelle.trim();
  const type = (input.type?.trim() || "autre") as string;
  if (!code || !libelle) throw new Error("Code et libellé obligatoires.");

  let anneeScolaireId = input.anneeScolaireId ?? null;
  if (!anneeScolaireId && !input.id) {
    const { resolveAnneeCouranteMeta } = await import("@/app/lib/annees-scolaires-db");
    anneeScolaireId = (await resolveAnneeCouranteMeta(etablissementId)).id;
  }

  if (input.id) {
    const patch: {
      code: string;
      libelle: string;
      type: string;
      updatedAt: Date;
      anneeScolaireId?: string | null;
    } = {
      code,
      libelle,
      type,
      updatedAt: new Date(),
    };
    if (input.anneeScolaireId !== undefined) {
      patch.anneeScolaireId = input.anneeScolaireId;
    }
    const [row] = await db
      .update(groupePedagogique)
      .set(patch)
      .where(
        and(eq(groupePedagogique.etablissementId, etablissementId), eq(groupePedagogique.id, input.id)),
      )
      .returning();
    return row;
  }

  const [row] = await db
    .insert(groupePedagogique)
    .values({
      etablissementId,
      code,
      libelle,
      type,
      anneeScolaireId,
    })
    .onConflictDoUpdate({
      target: [groupePedagogique.etablissementId, groupePedagogique.code],
      set: {
        libelle,
        type,
        anneeScolaireId,
        updatedAt: new Date(),
      },
    })
    .returning();
  return row;
}

export async function deleteGroupe(etablissementId: string, groupeId: string) {
  const db = getDb();
  await db
    .delete(groupePedagogique)
    .where(
      and(eq(groupePedagogique.etablissementId, etablissementId), eq(groupePedagogique.id, groupeId)),
    );
}

export async function addGroupeMembre(
  etablissementId: string,
  groupeId: string,
  eleveId: string,
): Promise<{ added: boolean }> {
  const db = getDb();
  const [g] = await db
    .select({ id: groupePedagogique.id })
    .from(groupePedagogique)
    .where(
      and(eq(groupePedagogique.etablissementId, etablissementId), eq(groupePedagogique.id, groupeId)),
    )
    .limit(1);
  if (!g) throw new Error("Groupe introuvable.");

  const [e] = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!e) throw new Error("Élève introuvable.");

  const inserted = await db
    .insert(groupePedagogiqueMembre)
    .values({ etablissementId, groupeId, eleveId })
    .onConflictDoNothing()
    .returning({ eleveId: groupePedagogiqueMembre.eleveId });

  return { added: inserted.length > 0 };
}

export async function removeGroupeMembre(
  etablissementId: string,
  groupeId: string,
  eleveId: string,
) {
  const db = getDb();
  await db
    .delete(groupePedagogiqueMembre)
    .where(
      and(
        eq(groupePedagogiqueMembre.etablissementId, etablissementId),
        eq(groupePedagogiqueMembre.groupeId, groupeId),
        eq(groupePedagogiqueMembre.eleveId, eleveId),
      ),
    );
}

export async function addGroupeMembresFromClasse(
  etablissementId: string,
  groupeId: string,
  classe: string,
): Promise<{ added: number; total: number }> {
  const db = getDb();
  const cls = classe.trim();
  if (!cls) throw new Error("Classe obligatoire.");

  const eleves = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        sql`lower(trim(${eleve.classe})) = lower(${cls})`,
      ),
    );

  let added = 0;
  for (const e of eleves) {
    const r = await addGroupeMembre(etablissementId, groupeId, e.id);
    if (r.added) added += 1;
  }

  return { added, total: eleves.length };
}

export async function upsertGroupesFromSiecleCodes(
  etablissementId: string,
  rows: { code: string; libelle: string }[],
): Promise<{ inserts: number; updates: number }> {
  let inserts = 0;
  let updates = 0;
  for (const row of rows) {
    const code = row.code.trim().toUpperCase();
    const libelle = row.libelle.trim();
    if (!code || !libelle) continue;
    const db = getDb();
    const [existing] = await db
      .select({ id: groupePedagogique.id })
      .from(groupePedagogique)
      .where(
        and(eq(groupePedagogique.etablissementId, etablissementId), eq(groupePedagogique.code, code)),
      )
      .limit(1);
    if (existing) {
      await db
        .update(groupePedagogique)
        .set({ libelle, type: "autre", updatedAt: new Date() })
        .where(eq(groupePedagogique.id, existing.id));
      updates += 1;
    } else {
      await db.insert(groupePedagogique).values({
        etablissementId,
        code,
        libelle,
        type: "autre",
      });
      inserts += 1;
    }
  }
  return { inserts, updates };
}

export async function searchElevesForGroupe(
  etablissementId: string,
  query: string,
  limit = 20,
): Promise<GroupeMembreRow[]> {
  const db = getDb();
  const q = query.trim();
  if (!q || q.length < 2) return [];

  const rows = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
      ine: eleve.ine,
    })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        sqlPersonNameMatches({
          nom: eleve.nom,
          prenom: eleve.prenom,
          extras: [eleve.classe, eleve.ine],
          query: q,
        }),
      ),
    )
    .limit(limit);

  return rows.map((r) => ({
    eleveId: r.id,
    nom: r.nom,
    prenom: r.prenom,
    classe: r.classe,
    ine: r.ine,
  }));
}
