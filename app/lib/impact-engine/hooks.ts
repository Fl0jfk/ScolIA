import "server-only";

import type { TravelsTrip } from "@/app/lib/travels-types";
import { travelsDbReady } from "@/app/lib/travel-db";
import { previewVoyageImpacts } from "./preview";
import { clearCreneauVideSignalsForTravel } from "./signals";
import type { VoyageImpactPreview } from "./types";

/**
 * Après confirm-liste (participants upsertés, statut souvent VALIDE).
 * Calcule A/B/C/D + signaux créneaux vidés. N’écrit pas le planning.
 */
export async function onTravelListeConfirmed(opts: {
  trip: TravelsTrip;
  etablissementId?: string | null;
}): Promise<VoyageImpactPreview | null> {
  const etabId = (opts.etablissementId || (await travelsDbReady()) || "").trim();
  if (!etabId || !opts.trip?.id) return null;

  try {
    return await previewVoyageImpacts({
      etablissementId: etabId,
      trip: opts.trip,
      mode: "as_if_active",
      persistSignals: true,
    });
  } catch (err) {
    console.error("[impact-engine] onTravelListeConfirmed", err);
    return null;
  }
}

/**
 * Après ANNULE / SEANCE_ANNULEE : retire les signaux du voyage.
 */
export async function onTravelCancelled(opts: {
  trip: TravelsTrip;
  etablissementId?: string | null;
}): Promise<VoyageImpactPreview | null> {
  const etabId = (opts.etablissementId || (await travelsDbReady()) || "").trim();
  if (!etabId || !opts.trip?.id) return null;

  try {
    clearCreneauVideSignalsForTravel(etabId, opts.trip.id);
    return await previewVoyageImpacts({
      etablissementId: etabId,
      trip: { ...opts.trip, status: opts.trip.status || "ANNULE" },
      mode: "as_if_active",
      persistSignals: true,
    });
  } catch (err) {
    console.error("[impact-engine] onTravelCancelled", err);
    return null;
  }
}
