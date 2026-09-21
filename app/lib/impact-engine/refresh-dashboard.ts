import "server-only";

import { listTravelsFromDb } from "@/app/lib/travel-db";
import { OCCUPANCY_TRAVEL_STATUSES } from "@/app/lib/occupancy/travels-read";
import { previewVoyageImpacts } from "./preview";
import { listCreneauVideSignals } from "./signals";
import type { CreneauVideSignal } from "./types";

const ACTIVE = new Set<string>(OCCUPANCY_TRAVEL_STATUSES);

/**
 * Dashboard / file VS : lit le store process, et si vide recompute à la volée
 * depuis les voyages actifs du jour (survit à un redémarrage Next).
 * N’écrit pas le planning.
 */
export async function loadCreneauVideSignalsForDashboard(opts: {
  etablissementId: string;
  date: string;
  /** Max voyages à rescanner si le store est vide. */
  maxTrips?: number;
}): Promise<CreneauVideSignal[]> {
  const etabId = opts.etablissementId.trim();
  if (!etabId) return [];

  const cached = listCreneauVideSignals(etabId);
  if (cached.length > 0) return cached;

  const date = opts.date.trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return [];

  try {
    const trips = await listTravelsFromDb(etabId);
    const active = trips.filter((t) => {
      if (!ACTIVE.has(String(t.status || ""))) return false;
      if (t.data?.listeElevesStatus !== "confirmed") return false;
      const start = String(t.data?.startDate || t.startDate || "").slice(0, 10);
      const end = String(t.data?.endDate || t.endDate || start).slice(0, 10);
      return start <= date && end >= date;
    });
    const cap = opts.maxTrips ?? 12;
    for (const trip of active.slice(0, cap)) {
      await previewVoyageImpacts({
        etablissementId: etabId,
        trip,
        mode: "as_if_active",
        persistSignals: true,
      });
    }
  } catch (err) {
    console.warn("[impact-engine] loadCreneauVideSignalsForDashboard", err);
  }

  return listCreneauVideSignals(etabId);
}
