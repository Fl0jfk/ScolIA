import { parseParisDateTime } from "@/app/lib/paris-time";
import type { PortesOuvertesSlot } from "@/app/lib/toolbox-types";

/** Fréquence des départs de groupes (toutes les X minutes). */
export type PortesOuvertesDepartureIntervalMinutes = 15 | 30 | 60;

/** Durée réelle d’une visite (indépendante du rythme des départs). */
export type PortesOuvertesVisitDurationMinutes = 30 | 45 | 60 | 75 | 90;

/** @deprecated Utiliser `PortesOuvertesDepartureIntervalMinutes`. */
export type PortesOuvertesSlotIntervalMinutes = PortesOuvertesDepartureIntervalMinutes;

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function formatSlotLabel(startHm: string, endHm: string): string {
  const pretty = (hm: string) => {
    const [h, m] = hm.split(":").map(Number);
    if (!m) return `${h} h`;
    return `${h} h ${pad2(m)}`;
  };
  return `${pretty(startHm)} – ${pretty(endHm)}`;
}

function addMinutesToHm(hm: string, minutes: number): string {
  const [h, m] = hm.split(":").map(Number);
  const total = (h || 0) * 60 + (m || 0) + minutes;
  const nh = Math.floor(total / 60);
  const nm = total % 60;
  return `${pad2(nh)}:${pad2(nm)}`;
}

function hmToMinutes(hm: string): number {
  const [h, m] = hm.split(":").map(Number);
  return (h || 0) * 60 + (m || 0);
}

/**
 * Génère une grille de départs pour une journée.
 *
 * - `departureIntervalMinutes` : rythme des départs (15 / 30 / 60 min).
 * - `visitDurationMinutes` : durée de la visite (souvent ~1 h), indépendante du rythme.
 *
 * Ex. 08:30 → 12:00, départ toutes les 15 min, visite 60 min →
 * 08:30–09:30, 08:45–09:45, 09:00–10:00, … (créneaux qui se chevauchent volontairement).
 *
 * `endTime` = heure de fin de la dernière visite (fermeture).
 */
export function generatePortesOuvertesSlots(params: {
  date: string;
  startTime: string;
  endTime: string;
  /** Rythme des départs. Alias historique : `intervalMinutes`. */
  departureIntervalMinutes?: PortesOuvertesDepartureIntervalMinutes;
  /** @deprecated Préférer `departureIntervalMinutes`. */
  intervalMinutes?: PortesOuvertesDepartureIntervalMinutes;
  visitDurationMinutes?: PortesOuvertesVisitDurationMinutes;
  maxPlaces?: number;
  cycle?: "ecole" | "college" | "lycee";
}): PortesOuvertesSlot[] {
  const date = params.date.trim();
  const startTime = params.startTime.trim();
  const endTime = params.endTime.trim();
  const departure =
    params.departureIntervalMinutes ?? params.intervalMinutes ?? 30;
  const visitDuration = params.visitDurationMinutes ?? departure;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return [];
  if (!/^\d{1,2}:\d{2}$/.test(startTime) || !/^\d{1,2}:\d{2}$/.test(endTime)) return [];
  if (departure <= 0 || visitDuration <= 0) return [];

  const startMin = hmToMinutes(startTime);
  const endMin = hmToMinutes(endTime);
  if (endMin <= startMin) return [];

  const slots: PortesOuvertesSlot[] = [];
  let cursor = startMin;
  let i = 0;
  const cycleSuffix = params.cycle ? `-${params.cycle}` : "";
  while (cursor + visitDuration <= endMin) {
    const startHm = `${pad2(Math.floor(cursor / 60))}:${pad2(cursor % 60)}`;
    const endHm = addMinutesToHm(startHm, visitDuration);
    const startDate = parseParisDateTime(date, startHm);
    const endDate = parseParisDateTime(date, endHm);
    if (!startDate || !endDate) break;
    const id = `slot-${date.replace(/-/g, "")}-${startHm.replace(":", "")}${cycleSuffix}-${i}-${Date.now().toString(36)}`;
    slots.push({
      id,
      label: formatSlotLabel(startHm, endHm),
      startAt: startDate.toISOString(),
      endAt: endDate.toISOString(),
      maxPlaces: params.maxPlaces && params.maxPlaces > 0 ? params.maxPlaces : undefined,
      cycle: params.cycle,
    });
    cursor += departure;
    i += 1;
  }
  return slots;
}
