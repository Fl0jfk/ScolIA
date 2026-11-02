import "server-only";

import { listTravelsFromDb } from "@/app/lib/travel-db";
import { runWithConcurrency } from "@/app/lib/async-concurrency";
import { filterTripsActiveOnParisDate } from "@/app/lib/impact-engine/empty-slot-trip-filter";
import { valkeyCached } from "@/app/lib/valkey";
import { VALKEY_TTL, valkeyKeyDashboardCreneauxVides } from "@/app/lib/valkey-keys";
import { previewVoyageImpacts } from "./preview";
import { listCreneauVideSignals } from "./signals";
import type { CreneauVideSignal } from "./types";

const PREVIEW_CONCURRENCY = 3;

export type CreneauVideDashboardPayload = {
  signals: CreneauVideSignal[];
};

/**
 * Dashboard / file VS : lit le store process, et si vide recompute à la volée
 * depuis les voyages actifs du jour (survit à un redémarrage Next).
 * N’écrit pas le planning.
 */
async function loadCreneauVideSignalsForDashboardUncached(opts: {
  etablissementId: string;
  date: string;
  maxTrips?: number;
}): Promise<CreneauVideSignal[]> {
  const etabId = opts.etablissementId.trim();
  if (!etabId) return [];

  const cached = listCreneauVideSignals(etabId);
  if (cached.length > 0) return cached;

  const date = opts.date.trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return [];

  try {
    const trips = await listTravelsFromDb(etabId, { forListIndex: true });
    const active = filterTripsActiveOnParisDate(trips, date);
    const cap = opts.maxTrips ?? 12;
    const slice = active.slice(0, cap);
    await runWithConcurrency(
      slice.map(
        (trip) => () =>
          previewVoyageImpacts({
            etablissementId: etabId,
            trip,
            mode: "as_if_active",
            persistSignals: true,
          }),
      ),
      PREVIEW_CONCURRENCY,
    );
  } catch (err) {
    console.warn("[impact-engine] loadCreneauVideSignalsForDashboard", err);
  }

  return listCreneauVideSignals(etabId);
}

/** Cache établissement + jour (y compris tableau vide) pour éviter rescan voyages à chaque hit dashboard. */
export async function loadCreneauVideSignalsForDashboard(opts: {
  etablissementId: string;
  date: string;
  maxTrips?: number;
}): Promise<CreneauVideSignal[]> {
  const etabId = opts.etablissementId.trim();
  const date = opts.date.trim();
  if (!etabId || !/^\d{4}-\d{2}-\d{2}$/.test(date)) return [];

  const payload = await valkeyCached({
    key: valkeyKeyDashboardCreneauxVides(etabId, date),
    ttlSeconds: VALKEY_TTL.dashboardCreneauxVides,
    loader: async (): Promise<CreneauVideDashboardPayload> => ({
      signals: await loadCreneauVideSignalsForDashboardUncached(opts),
    }),
  });
  return Array.isArray(payload.signals) ? payload.signals : [];
}
