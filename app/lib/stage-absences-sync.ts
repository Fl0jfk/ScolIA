import "server-only";

import type { StageConvention } from "@/app/lib/stage-types";

/**
 * Historique : créait une absence VS sur la période de stage.
 * Depuis 23 sept 2026 : le stage est un « ailleurs » (comme une sortie).
 * Occupancy lit la convention live (`en_stage`) — plus d’écriture `vs_absence_eleve`.
 * No-op conservé pour les appelants workflow (signatures, dépôt PDF…).
 */
export async function ensureStageAbsencesForConvention(
  convention: StageConvention,
): Promise<{ ok: true; absenceIds: string[]; skipped?: boolean } | { ok: false; error: string }> {
  return {
    ok: true,
    absenceIds: convention.stageAbsenceIds ?? [],
    skipped: true,
  };
}

/**
 * Resync dates des absences stage — no-op aligné sur ensureStageAbsencesForConvention.
 * Les anciennes lignes `vs_absence_eleve` éventuelles restent (pas de wipe) ;
 * l’appel / bulletin excluent désormais le tag `en_stage`.
 */
export async function resyncStageAbsencesForConvention(
  convention: StageConvention,
): Promise<{ ok: true; absenceIds: string[]; skipped?: boolean } | { ok: false; error: string }> {
  return ensureStageAbsencesForConvention(convention);
}
