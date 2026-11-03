import { parseParisDateTime } from "@/app/lib/paris-time";

const MANUAL_SLOT_MAX_MS = 4 * 60 * 60 * 1000;

/** Valide une saisie manuelle date + heures (heure de Paris). Pur, testable. */
export function parseManualRdvSlot(input: {
  date: string;
  startTime: string;
  endTime: string;
  now?: Date;
}): { ok: true; startAt: Date; endAt: Date } | { ok: false; error: string } {
  const date = String(input.date || "").trim();
  const startTime = String(input.startTime || "").trim();
  const endTime = String(input.endTime || "").trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return { ok: false, error: "Date invalide." };
  if (!/^\d{2}:\d{2}$/.test(startTime) || !/^\d{2}:\d{2}$/.test(endTime)) {
    return { ok: false, error: "Heure invalide (format HH:MM)." };
  }
  const startAt = parseParisDateTime(date, startTime);
  const endAt = parseParisDateTime(date, endTime);
  if (!startAt || !endAt || Number.isNaN(startAt.getTime()) || Number.isNaN(endAt.getTime())) {
    return { ok: false, error: "Date ou heure invalide." };
  }
  // Refuse les dates impossibles (ex. 31 février) qu'un calcul pourrait reporter au mois suivant.
  const parisDay = startAt.toLocaleDateString("en-CA", { timeZone: "Europe/Paris" });
  if (parisDay !== date) return { ok: false, error: "Date invalide." };
  if (endAt.getTime() <= startAt.getTime()) {
    return { ok: false, error: "L’heure de fin doit être après l’heure de début." };
  }
  if (endAt.getTime() - startAt.getTime() > MANUAL_SLOT_MAX_MS) {
    return { ok: false, error: "Durée trop longue (4 h maximum)." };
  }
  const now = input.now ?? new Date();
  if (startAt.getTime() < now.getTime() - 60_000) {
    return { ok: false, error: "Ce créneau est déjà passé." };
  }
  return { ok: true, startAt, endAt };
}
