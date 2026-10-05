/**
 * Règles créneau / appel — fonctions pures (tests sans BDD).
 */
import { absenceCoversSlot, timesOverlap } from "@/app/lib/accueil-absences-types";

export type AppelSlotRef = {
  dateAppel: string;
  heureDebut?: string | null;
  heureFin?: string | null;
  creneauId?: string | null;
};

export type ExistingAppelAbsence = {
  id: string;
  appelId: string | null;
  heureDebut?: string | null;
  heureFin?: string | null;
  statut: string;
};

/** Même élève, même jour, créneaux horaires qui se recouvrent (groupe + classe). */
export function appelAbsenceSlotsConflict(
  slot: AppelSlotRef,
  existing: ExistingAppelAbsence,
  currentAppelId: string,
): boolean {
  if (existing.statut === "classee") return false;
  if (existing.appelId === currentAppelId) return false;
  if (!existing.appelId) return false;

  return absenceCoversSlot({
    dateDebut: slot.dateAppel,
    dateFin: slot.dateAppel,
    heureDebut: slot.heureDebut,
    heureFin: slot.heureFin,
    slotDate: slot.dateAppel,
    slotHeureDebut: existing.heureDebut,
    slotHeureFin: existing.heureFin,
  }) &&
    timesOverlap(
      slot.heureDebut,
      slot.heureFin,
      existing.heureDebut,
      existing.heureFin,
    );
}

/** Ne pas DELETE l’absence si un motif / traitement existe déjà. */
export function shouldPreserveAbsenceOnPresentLine(absence: {
  motif?: string | null;
  justifie?: boolean;
  statut?: string;
  noteCpe?: string | null;
}): boolean {
  if (absence.justifie) return true;
  if (absence.motif?.trim()) return true;
  if (absence.noteCpe?.trim()) return true;
  const s = absence.statut || "a_traiter";
  return s !== "a_traiter";
}
