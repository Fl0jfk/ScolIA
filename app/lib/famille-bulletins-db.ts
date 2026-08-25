import "server-only";

import { and, asc, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import {
  anneeScolaire,
  eleve,
  noteCompetenceValeur,
  noteMoyenneEleve,
  notePeriode,
} from "@/db/schema";

export type FamilleBulletinSummary = {
  eleveId: string;
  eleveNom: string;
  elevePrenom: string;
  eleveClasse: string | null;
  periodeId: string;
  periodeCode: string;
  periodeLibelle: string;
  anneeLabel: string;
  moyenneGenerale: string | null;
  nbMatieres: number;
  nbCompetences: number;
  pdfUrl: string;
};

function computeMgFromRows(
  rows: Array<{ moyenne: string | null; coef: string; compteDansMg: boolean }>,
): string | null {
  let sum = 0;
  let totalCoef = 0;
  for (const r of rows) {
    if (!r.compteDansMg || r.moyenne == null) continue;
    const m = Number(r.moyenne);
    const c = Number(r.coef) || 1;
    if (!Number.isFinite(m)) continue;
    sum += m * c;
    totalCoef += c;
  }
  if (totalCoef <= 0) return null;
  return (sum / totalCoef).toFixed(2);
}

/** Bulletins publiés = périodes clôturées avec moyennes ou compétences LSU. */
export async function listFamilleBulletins(
  etablissementId: string,
  eleveIds: string[],
): Promise<FamilleBulletinSummary[]> {
  if (!eleveIds.length) return [];
  const db = getDb();

  const periodes = await db
    .select({
      id: notePeriode.id,
      code: notePeriode.code,
      libelle: notePeriode.libelle,
      anneeScolaireId: notePeriode.anneeScolaireId,
    })
    .from(notePeriode)
    .where(and(eq(notePeriode.etablissementId, etablissementId), eq(notePeriode.statut, "cloturee")))
    .orderBy(asc(notePeriode.ordre), asc(notePeriode.code));

  if (!periodes.length) return [];

  const anneeIds = [...new Set(periodes.map((p) => p.anneeScolaireId).filter(Boolean))] as string[];
  const anneeLabels = new Map<string, string>();
  if (anneeIds.length) {
    const annees = await db
      .select({ id: anneeScolaire.id, label: anneeScolaire.label })
      .from(anneeScolaire)
      .where(
        and(
          eq(anneeScolaire.etablissementId, etablissementId),
          inArray(anneeScolaire.id, anneeIds),
        ),
      );
    for (const a of annees) anneeLabels.set(a.id, a.label);
  }

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
  const periodeIds = periodes.map((p) => p.id);

  const moyennes = await db
    .select({
      eleveId: noteMoyenneEleve.eleveId,
      periodeId: noteMoyenneEleve.periodeId,
      moyenne: noteMoyenneEleve.moyenne,
    })
    .from(noteMoyenneEleve)
    .where(
      and(
        eq(noteMoyenneEleve.etablissementId, etablissementId),
        inArray(noteMoyenneEleve.eleveId, eleveIds),
        inArray(noteMoyenneEleve.periodeId, periodeIds),
      ),
    );

  const competences = await db
    .select({
      eleveId: noteCompetenceValeur.eleveId,
      periodeId: noteCompetenceValeur.periodeId,
      n: sql<number>`count(*)::int`,
    })
    .from(noteCompetenceValeur)
    .where(
      and(
        eq(noteCompetenceValeur.etablissementId, etablissementId),
        inArray(noteCompetenceValeur.eleveId, eleveIds),
        inArray(noteCompetenceValeur.periodeId, periodeIds),
      ),
    )
    .groupBy(noteCompetenceValeur.eleveId, noteCompetenceValeur.periodeId);

  const moyenneMap = new Map<string, typeof moyennes>();
  for (const m of moyennes) {
    const key = `${m.eleveId}:${m.periodeId}`;
    const list = moyenneMap.get(key) ?? [];
    list.push(m);
    moyenneMap.set(key, list);
  }

  const competenceMap = new Map<string, number>();
  for (const c of competences) {
    competenceMap.set(`${c.eleveId}:${c.periodeId}`, c.n);
  }

  const out: FamilleBulletinSummary[] = [];
  const seen = new Set<string>();

  for (const periode of periodes) {
    const anneeLabel = periode.anneeScolaireId
      ? anneeLabels.get(periode.anneeScolaireId) || "Année scolaire"
      : "Année scolaire";

    for (const eleveId of eleveIds) {
      const key = `${eleveId}:${periode.id}`;
      const mRows = moyenneMap.get(key) ?? [];
      const nbCompetences = competenceMap.get(key) ?? 0;
      if (!mRows.length && nbCompetences === 0) continue;
      if (seen.has(key)) continue;
      seen.add(key);

      const e = eleveById.get(eleveId);
      if (!e) continue;

      const nbMatieres = mRows.filter((r) => r.moyenne != null).length;
      const moyenneGenerale = computeMgFromRows(
        mRows.map((r) => ({
          moyenne: r.moyenne != null ? String(r.moyenne) : null,
          coef: "1",
          compteDansMg: true,
        })),
      );

      out.push({
        eleveId,
        eleveNom: e.nom,
        elevePrenom: e.prenom,
        eleveClasse: e.classe,
        periodeId: periode.id,
        periodeCode: periode.code,
        periodeLibelle: periode.libelle,
        anneeLabel,
        moyenneGenerale,
        nbMatieres,
        nbCompetences,
        pdfUrl: `/api/famille/bulletins/pdf?eleveId=${encodeURIComponent(eleveId)}&periodeId=${encodeURIComponent(periode.id)}`,
      });
    }
  }

  return out.sort((a, b) => {
    const p = b.periodeCode.localeCompare(a.periodeCode, "fr");
    if (p !== 0) return p;
    return `${a.eleveNom} ${a.elevePrenom}`.localeCompare(`${b.eleveNom} ${b.elevePrenom}`, "fr", {
      sensitivity: "base",
    });
  });
}
