/**
 * Garde-fou accueil ← occupancy (lot « cerveau partout »).
 * Pur / testable : un élève `en_sortie` n’est pas une absence bulletin.
 * La déclaration reste possible avec `force` (cas rare : sortie quittée tôt).
 */

import { eachIsoDateInclusive } from "@/app/lib/impact-engine/empty-slots";
import type { OccupancyFact } from "@/app/lib/occupancy/types";

export const ACCUEIL_EN_SORTIE_CODE = "ELEVE_EN_SORTIE" as const;

export type AccueilEnSortieHit = {
  date: string;
  tag: "en_sortie";
  source?: string;
  travelId?: string | null;
};

export function picksEnSortieHits(
  facts: Array<Pick<OccupancyFact, "date" | "tag" | "source" | "detail">>,
): AccueilEnSortieHit[] {
  const hits: AccueilEnSortieHit[] = [];
  for (const f of facts) {
    if (f.tag !== "en_sortie") continue;
    hits.push({
      date: f.date,
      tag: "en_sortie",
      source: f.source,
      travelId: f.detail?.travelId ?? null,
    });
  }
  return hits;
}

/** Message court pour toast / bandeau accueil. */
export function formatAccueilEnSortieWarning(hits: AccueilEnSortieHit[]): string {
  if (hits.length === 0) return "";
  const dates = [...new Set(hits.map((h) => h.date))].sort();
  const datePart =
    dates.length === 1
      ? `le ${dates[0]}`
      : dates.length <= 3
        ? `les ${dates.join(", ")}`
        : `sur ${dates.length} jours (dont ${dates[0]} → ${dates[dates.length - 1]})`;
  return (
    `Cet élève est en sortie scolaire ${datePart} — ce n’est pas une absence bulletin. ` +
    `Enregistrer quand même seulement si vous savez qu’il n’est plus en sortie.`
  );
}

/** Texte confirm navigateur avant force. */
export function formatAccueilEnSortieConfirm(hits: AccueilEnSortieHit[]): string {
  const warn = formatAccueilEnSortieWarning(hits);
  return `${warn}\n\nForcer l’enregistrement de l’absence / retard ?`;
}

export function periodDatesForAccueilCheck(startDate: string, endDate: string): string[] {
  return eachIsoDateInclusive(startDate, endDate || startDate);
}

export class AccueilEnSortieConflictError extends Error {
  readonly code = ACCUEIL_EN_SORTIE_CODE;
  readonly hits: AccueilEnSortieHit[];

  constructor(hits: AccueilEnSortieHit[]) {
    super(formatAccueilEnSortieWarning(hits) || "Élève en sortie scolaire.");
    this.name = "AccueilEnSortieConflictError";
    this.hits = hits;
  }
}
