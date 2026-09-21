/**
 * Moteur d’impacts A/B/C/D (architecture §8 / §14.4).
 * Code déterministe — le LLM narre, il ne décide pas.
 */

export const IMPACT_TIROIRS = ["A", "B", "C", "D"] as const;
export type ImpactTiroir = (typeof IMPACT_TIROIRS)[number];

export type ImpactItem = {
  domaine: string;
  constat: string;
  tiroir: ImpactTiroir;
  /** Question bornée (tiroir C) ou couverture manquante (D). */
  question?: string;
  /** Couverture donnée (surtout D). */
  coverage?: "complete" | "partial" | "unavailable";
};

/** Créneau dont tous les élèves attendus sont `en_sortie`. */
export type CreneauVideSignal = {
  id: string;
  etablissementId: string;
  travelId: string;
  date: string;
  creneauId: string;
  jourSemaine: number;
  heureDebut: string;
  heureFin: string;
  classe: string | null;
  groupeId: string | null;
  enseignantNom: string | null;
  matiereLabel: string | null;
  expectedCount: number;
  enSortieCount: number;
  kind: "creneau_vide";
  createdAt: string;
};

export type VoyageImpactPreview = {
  etablissementId: string;
  travelId: string;
  title: string | null;
  dates: { start: string; end: string };
  status: string;
  participantCount: number;
  participantLinkedCount: number;
  panierRepasCount: number;
  impacts: ImpactItem[];
  creneauxVides: CreneauVideSignal[];
  /** Jamais d’écriture planning — rappel explicite pour tests / IA. */
  wroteTeacherPlanningReplacement: false;
  coverage: "complete" | "partial" | "unavailable";
};

export type CreneauPopulationInput = {
  creneauId: string;
  jourSemaine: number;
  heureDebut: string;
  heureFin: string;
  classe: string | null;
  groupeId: string | null;
  enseignantNom: string | null;
  matiereLabel?: string | null;
  /** Élèves attendus sur ce créneau (classe ou groupe). */
  expectedEleveIds: string[];
};
