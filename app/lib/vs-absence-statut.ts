/**
 * Statuts `vs_absence_eleve` — règles de visibilité / comptage (fonctions pures).
 */

export const VS_ABSENCE_STATUT_ANNULEE = "annulee";

export type AbsenceStatut =
  | "a_traiter"
  | "justifiee"
  | "non_justifiee"
  | "classee"
  | typeof VS_ABSENCE_STATUT_ANNULEE;

/** Déclaration annulée (ex. accueil) — conservée en BDD pour traçabilité uniquement. */
export function isVsAbsenceAnnulee(statut: string): boolean {
  return statut === VS_ABSENCE_STATUT_ANNULEE;
}

/**
 * Absence à afficher dans les listes, compteurs, occupancy, exports, espace famille/élève.
 * Les lignes `annulee` sont exclues ; `classee` reste une absence traitée (historique).
 */
export function isVsAbsenceVisibleEtComptee(statut: string): boolean {
  return !isVsAbsenceAnnulee(statut);
}

/**
 * Déclaration accueil encore « active » pour recouvrement créneau / tableau du jour.
 * (Ancien bug : `classee` + note annulation — migré vers `annulee`.)
 */
export function isVsAbsenceAccueilSlotActive(statut: string): boolean {
  return statut !== "classee" && !isVsAbsenceAnnulee(statut);
}

/** Conflit entre absences issues de feuilles d'appel sur le même créneau. */
export function isVsAbsenceActivePourConflitAppel(statut: string): boolean {
  return statut !== "classee" && !isVsAbsenceAnnulee(statut);
}

export function filterVsAbsencesVisiblesEtComptees<T extends { statut: string }>(rows: T[]): T[] {
  return rows.filter((r) => isVsAbsenceVisibleEtComptee(r.statut));
}
