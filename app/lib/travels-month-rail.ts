import {
  isTripTravelDatePast,
  tripTravelEndMs,
  tripTravelStartMs,
} from "./travels-trip-helpers";
import type { TravelsTripData } from "./travels-types";

/** Frise des mois du module Voyage : une entrée par mois, dans l'ordre de la liste. */

export type TravelMonthRailSection = "upcoming" | "past";

export type TravelMonthRailEntry = {
  /** Clé unique : section + mois (ex. "upcoming:2026-11", "past:none"). */
  key: string;
  section: TravelMonthRailSection;
  /** Année du mois, null si date à préciser. */
  year: number | null;
  /** 0-11, null si date à préciser. */
  month: number | null;
  /** Libellé court : "Novembre", ou "Date à préciser". */
  label: string;
  /** Libellé compact pour mobile : "nov. 2026". */
  shortLabel: string;
  count: number;
  /** Id du premier séjour de ce mois dans la liste (cible du clic). */
  firstTripId: string;
  /** Mois en cours (aujourd'hui). */
  isCurrentMonth: boolean;
};

type RailTrip = { id: string; type?: string; data?: TravelsTripData };

const MONTHS_FR = [
  "Janvier",
  "Février",
  "Mars",
  "Avril",
  "Mai",
  "Juin",
  "Juillet",
  "Août",
  "Septembre",
  "Octobre",
  "Novembre",
  "Décembre",
];

const MONTHS_FR_SHORT = [
  "janv.",
  "févr.",
  "mars",
  "avr.",
  "mai",
  "juin",
  "juil.",
  "août",
  "sept.",
  "oct.",
  "nov.",
  "déc.",
];

/** Clé de frise d'un séjour (mois de départ ; à défaut, mois de fin). */
export function travelMonthRailKey(trip: RailTrip): string {
  const section: TravelMonthRailSection = isTripTravelDatePast(trip) ? "past" : "upcoming";
  const ms = tripTravelStartMs(trip) ?? tripTravelEndMs(trip);
  if (ms == null) return `${section}:none`;
  const d = new Date(ms);
  return `${section}:${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
}

/**
 * Construit la frise à partir de la liste déjà triée (à venir puis terminés).
 * Les séjours consécutifs d'un même mois forment une seule entrée.
 */
export function buildTravelMonthRail(
  trips: RailTrip[],
  now: Date = new Date(),
): TravelMonthRailEntry[] {
  const entries: TravelMonthRailEntry[] = [];
  const byKey = new Map<string, TravelMonthRailEntry>();

  for (const trip of trips) {
    const key = travelMonthRailKey(trip);
    const existing = byKey.get(key);
    if (existing) {
      existing.count += 1;
      continue;
    }
    const [sectionRaw, monthRaw] = key.split(":");
    const section = sectionRaw as TravelMonthRailSection;
    let year: number | null = null;
    let month: number | null = null;
    if (monthRaw && monthRaw !== "none") {
      const [y, m] = monthRaw.split("-");
      year = Number(y);
      month = Number(m) - 1;
    }
    const entry: TravelMonthRailEntry = {
      key,
      section,
      year,
      month,
      label: month == null ? "Date à préciser" : MONTHS_FR[month]!,
      shortLabel:
        month == null || year == null ? "À préciser" : `${MONTHS_FR_SHORT[month]} ${year}`,
      count: 1,
      firstTripId: trip.id,
      isCurrentMonth: year === now.getFullYear() && month === now.getMonth(),
    };
    byKey.set(key, entry);
    entries.push(entry);
  }

  return entries;
}

/** Id DOM de l'ancre d'une carte séjour. */
export function travelCardAnchorId(tripId: string): string {
  return `travel-card-${tripId}`;
}
