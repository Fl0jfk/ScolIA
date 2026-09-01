import "server-only";

import type { TravelsTrip } from "@/app/lib/travels-types";
import {
  getTravelFromDb,
  listTravelsFromDb,
  travelsDbReady,
  upsertTravelInDb,
} from "@/app/lib/travel-db";

export async function listTravelsIndex(): Promise<TravelsTrip[]> {
  const etabId = await travelsDbReady();
  if (!etabId) return [];
  return listTravelsFromDb(etabId);
}

/** @deprecated Index JSON obsolète — préférer saveTravelTrip. Conservé pour compat. */
export async function saveTravelsIndex(index: TravelsTrip[]): Promise<void> {
  const etabId = await travelsDbReady();
  if (!etabId) throw new Error("[travels] Postgres requis");
  for (const t of index) {
    if (t?.id) await upsertTravelInDb(etabId, t);
  }
}

export async function getTravelTrip(id: string): Promise<TravelsTrip | null> {
  const etabId = await travelsDbReady();
  if (!etabId) return null;
  return getTravelFromDb(etabId, id);
}

export async function saveTravelTrip(trip: TravelsTrip): Promise<TravelsTrip> {
  const etabId = await travelsDbReady();
  if (!etabId) throw new Error("[travels] Postgres requis");
  await upsertTravelInDb(etabId, trip);
  return trip;
}
