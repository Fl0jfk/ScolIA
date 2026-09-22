/**
 * Feuille du jour — présentation occupancy pour l’écran staff « où est X ? ».
 * Pur / testable. Zéro écriture. Sortie ≠ absence bulletin.
 */

import { explainOccupancyTag } from "@/app/lib/brain-ai/core-read";
import type { OccupancyFact, OccupancyTag } from "@/app/lib/occupancy/types";
import { isExcludedFromBulletinAbsence } from "@/app/lib/occupancy/types";
import { occupancyTagToAppelBadge } from "@/app/lib/vs-appel-occupancy";

export type FeuilleDuJourRow = {
  eleveId: string;
  nom: string;
  prenom: string;
  displayName: string;
  classe: string | null;
  tag: OccupancyTag;
  labelFr: string;
  explanationFr: string;
  excludeFromBulletin: boolean;
  source?: string;
  detailFr?: string;
  travelId?: string | null;
};

export type FeuilleDuJourSummary = {
  total: number;
  enSortie: number;
  absentVs: number;
  enStage: number;
  aInfirmerie: number;
  horsEtablissement: number;
  autres: number;
};

export function buildFeuilleDuJourRow(opts: {
  eleveId: string;
  nom: string;
  prenom: string;
  classe?: string | null;
  fact: Pick<OccupancyFact, "eleveId" | "tag" | "source" | "detail">;
}): FeuilleDuJourRow {
  const badge = occupancyTagToAppelBadge(opts.fact);
  const nom = opts.nom.trim();
  const prenom = opts.prenom.trim();
  return {
    eleveId: opts.eleveId,
    nom,
    prenom,
    displayName: `${prenom} ${nom}`.trim() || nom || prenom || opts.eleveId,
    classe: opts.classe?.trim() || null,
    tag: badge.tag,
    labelFr: badge.labelFr,
    explanationFr: explainOccupancyTag(badge.tag),
    excludeFromBulletin: badge.excludeFromBulletin,
    source: badge.source,
    detailFr: badge.detailFr,
    travelId: opts.fact.detail?.travelId ?? null,
  };
}

export function summarizeFeuilleDuJour(rows: FeuilleDuJourRow[]): FeuilleDuJourSummary {
  let enSortie = 0;
  let absentVs = 0;
  let enStage = 0;
  let aInfirmerie = 0;
  let horsEtablissement = 0;
  let autres = 0;
  for (const r of rows) {
    if (r.tag === "en_sortie") enSortie += 1;
    else if (r.tag === "absent_vs") absentVs += 1;
    else if (r.tag === "en_stage") enStage += 1;
    else if (r.tag === "a_infirmerie") aInfirmerie += 1;
    else if (r.tag === "hors_etablissement") horsEtablissement += 1;
    else autres += 1;
  }
  return {
    total: rows.length,
    enSortie,
    absentVs,
    enStage,
    aInfirmerie,
    horsEtablissement,
    autres,
  };
}

/** Ordre d’affichage : anomalies / hors les murs d’abord, en_cours à la fin. */
export function sortFeuilleDuJourRows(rows: FeuilleDuJourRow[]): FeuilleDuJourRow[] {
  const rank = (tag: OccupancyTag): number => {
    switch (tag) {
      case "en_sortie":
        return 0;
      case "en_stage":
        return 1;
      case "a_infirmerie":
        return 2;
      case "absent_vs":
        return 3;
      case "hors_etablissement":
        return 4;
      case "internat":
        return 5;
      case "inconnu":
        return 6;
      case "en_cours":
        return 7;
      default:
        return 8;
    }
  };
  return [...rows].sort((a, b) => {
    const d = rank(a.tag) - rank(b.tag);
    if (d !== 0) return d;
    return a.displayName.localeCompare(b.displayName, "fr", { sensitivity: "base" });
  });
}

export function assertSortieNotBulletin(tag: OccupancyTag): boolean {
  return isExcludedFromBulletinAbsence(tag) === (tag === "en_sortie" || tag === "a_infirmerie");
}
