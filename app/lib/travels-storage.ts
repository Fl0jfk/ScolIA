import "server-only";

import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  deleteTravelFromDb,
  getTravelFromDb,
  listTravelsFromDb,
  travelsDbReady,
  upsertTravelInDb,
} from "@/app/lib/travel-db";
import { valkeyCached, valkeyDel, valkeyGetJson, valkeySetJson } from "@/app/lib/valkey";
import {
  VALKEY_TTL,
  valkeyKeyTravelTrip,
  valkeyKeyTravelsIndex,
} from "@/app/lib/valkey-keys";

export async function invalidateTravelsCaches(
  etablissementId: string,
  tripId?: string,
): Promise<void> {
  const id = etablissementId.trim();
  if (!id) return;
  await valkeyDel(valkeyKeyTravelsIndex(id));
  if (tripId?.trim()) {
    await valkeyDel(valkeyKeyTravelTrip(id, tripId.trim()));
  }
}

export async function listTravelsIndex(): Promise<TravelsTrip[]> {
  const etabId = await travelsDbReady();
  if (!etabId) return [];
  return valkeyCached({
    key: valkeyKeyTravelsIndex(etabId),
    ttlSeconds: VALKEY_TTL.travelsIndex,
    loader: () => listTravelsFromDb(etabId),
  });
}

/** @deprecated Index JSON obsolète — préférer saveTravelTrip. Conservé pour compat. */
export async function saveTravelsIndex(index: TravelsTrip[]): Promise<void> {
  const etabId = await travelsDbReady();
  if (!etabId) throw new Error("[travels] Postgres requis");
  for (const t of index) {
    if (t?.id) await upsertTravelInDb(etabId, t);
  }
  await invalidateTravelsCaches(etabId);
}

export async function getTravelTrip(id: string): Promise<TravelsTrip | null> {
  const tripId = id.trim();
  if (!tripId) return null;
  const etabId = await travelsDbReady();
  if (!etabId) return null;

  const cached = await valkeyGetJson<TravelsTrip>(valkeyKeyTravelTrip(etabId, tripId));
  if (cached && typeof cached === "object" && cached.id) {
    return cached;
  }

  const trip = await getTravelFromDb(etabId, tripId);
  if (trip) {
    void valkeySetJson(
      valkeyKeyTravelTrip(etabId, tripId),
      trip,
      VALKEY_TTL.travelsTrip,
    );
  }
  return trip;
}

export async function saveTravelTrip(trip: TravelsTrip): Promise<TravelsTrip> {
  const etabId = await travelsDbReady();
  if (!etabId) throw new Error("[travels] Postgres requis");
  await upsertTravelInDb(etabId, trip);
  // upsert invalide l’index + trip ; write-through pour le prochain GET détail.
  void valkeySetJson(
    valkeyKeyTravelTrip(etabId, trip.id),
    trip,
    VALKEY_TTL.travelsTrip,
  );
  return trip;
}

export async function deleteTravelTrip(id: string): Promise<void> {
  const tripId = id.trim();
  if (!tripId) return;
  const etabId = await travelsDbReady();
  if (!etabId) throw new Error("[travels] Postgres requis");
  await deleteTravelFromDb(etabId, tripId);
}
