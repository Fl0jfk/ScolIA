import "server-only";

import { and, desc, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, santeExtrait } from "@/db/schema";
import { sqlPersonNameMatches } from "@/app/lib/person-name-search";
import {
  isSanteExtraitPortee,
  type SanteExtraitPortee,
  type SanteExtraitRow,
} from "@/app/lib/sante-extraits-shared";

export {
  SANTE_EXTRAIT_PORTEES,
  SANTE_EXTRAIT_PORTEE_LABELS,
  isSanteExtraitPortee,
  type SanteExtraitPortee,
  type SanteExtraitRow,
} from "@/app/lib/sante-extraits-shared";

export async function listSanteExtraits(
  etablissementId: string,
  opts?: { portee?: string; eleveId?: string; actifsSeulement?: boolean },
): Promise<SanteExtraitRow[]> {
  const db = getDb();
  const conditions = [eq(santeExtrait.etablissementId, etablissementId)];
  if (opts?.portee && isSanteExtraitPortee(opts.portee)) {
    conditions.push(eq(santeExtrait.portee, opts.portee));
  }
  if (opts?.eleveId) {
    conditions.push(eq(santeExtrait.eleveId, opts.eleveId));
  }
  if (opts?.actifsSeulement !== false) {
    conditions.push(eq(santeExtrait.actif, true));
  }

  const rows = await db
    .select({
      id: santeExtrait.id,
      eleveId: santeExtrait.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      portee: santeExtrait.portee,
      libelle: santeExtrait.libelle,
      actif: santeExtrait.actif,
      documentId: santeExtrait.documentId,
    })
    .from(santeExtrait)
    .innerJoin(
      eleve,
      and(eq(eleve.id, santeExtrait.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(and(...conditions))
    .orderBy(desc(santeExtrait.updatedAt))
    .limit(200);

  return rows.map((r) => ({
    id: r.id,
    eleveId: r.eleveId,
    eleveNom: r.eleveNom,
    elevePrenom: r.elevePrenom,
    eleveClasse: r.eleveClasse,
    portee: r.portee as SanteExtraitPortee,
    libelle: r.libelle,
    actif: r.actif,
    documentId: r.documentId,
  }));
}

/** Lecture bornée pour cantine / voyage / EPS — pas le dossier médical. */
export async function listSanteExtraitsPourPortee(
  etablissementId: string,
  portee: SanteExtraitPortee,
  eleveIds?: string[],
): Promise<Array<{ eleveId: string; libelle: string }>> {
  const db = getDb();
  const conditions = [
    eq(santeExtrait.etablissementId, etablissementId),
    eq(santeExtrait.portee, portee),
    eq(santeExtrait.actif, true),
  ];
  if (eleveIds && eleveIds.length > 0) {
    const { inArray } = await import("drizzle-orm");
    conditions.push(inArray(santeExtrait.eleveId, eleveIds));
  }
  const rows = await db
    .select({
      eleveId: santeExtrait.eleveId,
      libelle: santeExtrait.libelle,
    })
    .from(santeExtrait)
    .where(and(...conditions));
  return rows;
}

export async function createSanteExtrait(
  etablissementId: string,
  opts: { eleveId: string; portee: string; libelle: string; documentId?: string | null },
): Promise<SanteExtraitRow> {
  const db = getDb();
  const eleveId = opts.eleveId.trim();
  const libelle = opts.libelle.trim();
  if (!eleveId) throw new Error("Élève requis.");
  if (!libelle) throw new Error("Libellé requis.");
  if (!isSanteExtraitPortee(opts.portee)) throw new Error("Portée invalide.");

  const [eleveRow] = await db
    .select({
      id: eleve.id,
      nom: eleve.nom,
      prenom: eleve.prenom,
      classe: eleve.classe,
    })
    .from(eleve)
    .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.id, eleveId)))
    .limit(1);
  if (!eleveRow) throw new Error("Élève introuvable.");

  const [row] = await db
    .insert(santeExtrait)
    .values({
      etablissementId,
      eleveId,
      portee: opts.portee,
      libelle,
      documentId: opts.documentId ?? null,
      actif: true,
    })
    .returning({
      id: santeExtrait.id,
      eleveId: santeExtrait.eleveId,
      portee: santeExtrait.portee,
      libelle: santeExtrait.libelle,
      actif: santeExtrait.actif,
      documentId: santeExtrait.documentId,
    });

  return {
    id: row!.id,
    eleveId: row!.eleveId,
    eleveNom: eleveRow.nom,
    elevePrenom: eleveRow.prenom,
    eleveClasse: eleveRow.classe,
    portee: row!.portee as SanteExtraitPortee,
    libelle: row!.libelle,
    actif: row!.actif,
    documentId: row!.documentId,
  };
}

/** Soft : désactive. Pas de DELETE. */
export async function desactiverSanteExtrait(
  etablissementId: string,
  id: string,
): Promise<void> {
  const db = getDb();
  const [existing] = await db
    .select({ id: santeExtrait.id })
    .from(santeExtrait)
    .where(and(eq(santeExtrait.etablissementId, etablissementId), eq(santeExtrait.id, id)))
    .limit(1);
  if (!existing) throw new Error("Extrait introuvable.");
  await db
    .update(santeExtrait)
    .set({ actif: false, updatedAt: new Date() })
    .where(and(eq(santeExtrait.etablissementId, etablissementId), eq(santeExtrait.id, id)));
}

export async function searchElevesForSanteExtrait(
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
    .limit(20);
}
