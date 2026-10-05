import "server-only";

import { and, desc, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, passage } from "@/db/schema";
import { sqlPersonNameMatches } from "@/app/lib/person-name-search";
import { listSanteExtraitsPourPortee } from "@/app/lib/sante-extraits-db";
import {
  isPassageLieu,
  isPassageSens,
  type PassageLieu,
  type PassageRow,
  type PassageSens,
} from "@/app/lib/passages-shared";

export {
  PASSAGE_LIEUX,
  PASSAGE_LIEU_LABELS,
  PASSAGE_SENS,
  PASSAGE_SENS_LABELS,
  isPassageLieu,
  isPassageSens,
  type PassageLieu,
  type PassageRow,
  type PassageSens,
} from "@/app/lib/passages-shared";

function toIso(d: Date | string): string {
  if (typeof d === "string") return d;
  return d.toISOString();
}

function mapRow(r: {
  id: string;
  eleveId: string | null;
  eleveNom: string | null;
  elevePrenom: string | null;
  eleveClasse: string | null;
  inviteNom: string | null;
  sens: string;
  lieu: string;
  horodatage: Date | string;
  source: string;
}): PassageRow {
  return {
    id: r.id,
    eleveId: r.eleveId,
    eleveNom: r.eleveNom,
    elevePrenom: r.elevePrenom,
    eleveClasse: r.eleveClasse,
    inviteNom: r.inviteNom,
    sens: isPassageSens(r.sens) ? r.sens : "entree",
    lieu: isPassageLieu(r.lieu) ? r.lieu : "autre",
    horodatage: toIso(r.horodatage),
    source: r.source,
  };
}

/** Passages du jour civil (UTC date string YYYY-MM-DD). */
export async function listPassagesDuJour(
  etablissementId: string,
  opts?: { date?: string; lieu?: string; limit?: number },
): Promise<PassageRow[]> {
  const db = getDb();
  const date = (opts?.date ?? new Date().toISOString().slice(0, 10)).slice(0, 10);
  const dayStart = `${date}T00:00:00.000Z`;
  const dayEnd = `${date}T23:59:59.999Z`;

  const conditions = [
    eq(passage.etablissementId, etablissementId),
    sql`${passage.horodatage} >= ${dayStart}::timestamptz`,
    sql`${passage.horodatage} <= ${dayEnd}::timestamptz`,
  ];
  if (opts?.lieu && isPassageLieu(opts.lieu)) {
    conditions.push(eq(passage.lieu, opts.lieu));
  }

  const rows = await db
    .select({
      id: passage.id,
      eleveId: passage.eleveId,
      eleveNom: eleve.nom,
      elevePrenom: eleve.prenom,
      eleveClasse: eleve.classe,
      inviteNom: passage.inviteNom,
      sens: passage.sens,
      lieu: passage.lieu,
      horodatage: passage.horodatage,
      source: passage.source,
    })
    .from(passage)
    .leftJoin(
      eleve,
      and(eq(eleve.id, passage.eleveId), eq(eleve.etablissementId, etablissementId)),
    )
    .where(and(...conditions))
    .orderBy(desc(passage.horodatage))
    .limit(opts?.limit ?? 200);

  return rows.map(mapRow);
}

export async function searchElevesForPassage(
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

/** Alertes cantine (extraits) pour un élève — à afficher au self. */
export async function getAlertesCantinePourEleve(
  etablissementId: string,
  eleveId: string,
): Promise<string[]> {
  const rows = await listSanteExtraitsPourPortee(etablissementId, "cantine", [eleveId]);
  return rows.map((r) => r.libelle);
}

/**
 * Enregistre un passage. Insert unitaire — pas de DELETE.
 * Self : sens typique = entrée (repas pris). Portail : entrée ou sortie.
 */
export async function createPassage(
  etablissementId: string,
  opts: {
    eleveId?: string | null;
    inviteNom?: string | null;
    sens: string;
    lieu: string;
    source?: string;
    horodatage?: Date;
  },
): Promise<PassageRow & { alertesCantine?: string[] }> {
  const db = getDb();
  if (!isPassageSens(opts.sens)) throw new Error("Sens invalide (entrée / sortie).");
  if (!isPassageLieu(opts.lieu)) throw new Error("Lieu invalide.");

  const eleveId = opts.eleveId?.trim() || null;
  const inviteNom = opts.inviteNom?.trim() || null;
  if (!eleveId && !inviteNom) throw new Error("Élève ou invité requis.");

  let eleveNom: string | null = null;
  let elevePrenom: string | null = null;
  let eleveClasse: string | null = null;

  if (eleveId) {
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
    eleveNom = eleveRow.nom;
    elevePrenom = eleveRow.prenom;
    eleveClasse = eleveRow.classe;
  }

  const horodatage = opts.horodatage ?? new Date();
  const [row] = await db
    .insert(passage)
    .values({
      etablissementId,
      eleveId,
      inviteNom,
      sens: opts.sens,
      lieu: opts.lieu,
      horodatage,
      source: (opts.source ?? "manuel").trim() || "manuel",
    })
    .returning({
      id: passage.id,
      eleveId: passage.eleveId,
      inviteNom: passage.inviteNom,
      sens: passage.sens,
      lieu: passage.lieu,
      horodatage: passage.horodatage,
      source: passage.source,
    });

  const mapped = mapRow({
    ...row!,
    eleveNom,
    elevePrenom,
    eleveClasse,
  });

  let alertesCantine: string[] | undefined;
  if (eleveId && opts.lieu === "self") {
    alertesCantine = await getAlertesCantinePourEleve(etablissementId, eleveId);
  }

  return { ...mapped, alertesCantine };
}
