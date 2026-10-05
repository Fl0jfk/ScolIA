/**
 * Occupancy — tags de présence live (architecture §14.3 / capability get_presence_jour).
 * Jamais fusionner `en_sortie` et `absent_vs`. Zéro écriture.
 */

export const OCCUPANCY_TAGS = [
  "en_sortie",
  "en_stage",
  "a_infirmerie",
  "absent_vs",
  "hors_etablissement",
  "internat",
  "en_cours",
  "inconnu",
] as const;

export type OccupancyTag = (typeof OCCUPANCY_TAGS)[number];

export type OccupancyCoverage = "complete" | "partial" | "unavailable";

export type OccupancyConfidence = "defined" | "deduced" | "unknown";

/** Priorité décroissante : la première gagne. Sortie / stage ≠ bulletin. Infirmerie ≠ absence. */
export const OCCUPANCY_TAG_PRIORITY: readonly OccupancyTag[] = [
  "en_sortie",
  "en_stage",
  "a_infirmerie",
  "absent_vs",
  "hors_etablissement",
  "internat",
  "en_cours",
  "inconnu",
];

export type OccupancySignal = {
  eleveId: string;
  tag: OccupancyTag;
  source: string;
  coverage?: OccupancyCoverage;
  confidenceTag?: OccupancyConfidence;
  detail?: OccupancyFactDetail;
};

export type OccupancyFactDetail = {
  travelId?: string;
  absenceId?: string;
  motif?: string | null;
  /** Classe figée sur le participant voyage (document). */
  snapshotClasse?: string | null;
  /** Classe live scolarité / plat. */
  liveClasse?: string | null;
  passageId?: string;
  infirmeriePassageId?: string;
  lieu?: string | null;
  sens?: string | null;
};

export type OccupancyFact = {
  eleveId: string;
  date: string;
  tag: OccupancyTag;
  source: string;
  coverage: OccupancyCoverage;
  confidenceTag: OccupancyConfidence;
  detail?: OccupancyFactDetail;
};

export type UnmatchedTravelParticipant = {
  travelId: string;
  eleveKey: string;
  nom: string;
  prenom: string;
  snapshotClasse: string | null;
};

export type OccupancyResult = {
  etablissementId: string;
  date: string;
  facts: OccupancyFact[];
  unmatchedParticipants: UnmatchedTravelParticipant[];
  coverage: OccupancyCoverage;
};

export function isExcludedFromBulletinAbsence(tag: OccupancyTag): boolean {
  return tag === "en_sortie" || tag === "en_stage" || tag === "a_infirmerie";
}

export function isStageAbsenceMotif(motif: string | null | undefined): boolean {
  const m = (motif ?? "").trim();
  return /^Stage\s*[—–-]/i.test(m);
}

export function tagPriorityIndex(tag: OccupancyTag): number {
  const i = OCCUPANCY_TAG_PRIORITY.indexOf(tag);
  return i < 0 ? OCCUPANCY_TAG_PRIORITY.length : i;
}
