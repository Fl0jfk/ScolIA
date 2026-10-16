import { OCCUPANCY_TRAVEL_STATUSES } from "@/app/lib/occupancy/travels-read";
import type { TravelsTrip } from "@/app/lib/travels-types";

const ACTIVE = new Set<string>(OCCUPANCY_TRAVEL_STATUSES);

/** Filtre voyages actifs couvrant une date — même logique que refresh-dashboard (liste index). */
export function filterTripsActiveOnParisDate(
  trips: TravelsTrip[],
  date: string,
): TravelsTrip[] {
  const day = date.trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(day)) return [];
  return trips.filter((t) => {
    if (!ACTIVE.has(String(t.status || ""))) return false;
    if (t.data?.listeElevesStatus !== "confirmed") return false;
    const start = String(t.data?.startDate || "").slice(0, 10);
    const end = String(t.data?.endDate || start).slice(0, 10);
    return start <= day && end >= day;
  });
}
