import { NextResponse } from "next/server";
import { requireAuth } from "@/app/lib/intranet-auth";
import {
  compareTripsByTravelDate,
  isTripEligibleForPurge,
} from "@/app/lib/travels-trip-helpers";
import { normalizeTripImageFields } from "@/app/lib/travels-image-url";
import type { TravelsTrip } from "@/app/lib/travels-types";
import { listTravelsIndex, saveTravelsIndex } from "@/app/lib/travels-storage";
import { deleteTravelFromDb, travelsDbReady } from "@/app/lib/travel-db";

async function purgeOldTrips(trips: TravelsTrip[]): Promise<TravelsTrip[]> {
  const expired = trips.filter(isTripEligibleForPurge);
  if (expired.length === 0) return trips;

  const expiredIds = new Set(expired.map((t) => String(t.id)));
  const etabId = await travelsDbReady();
  await Promise.all(
    expired.map(async (t) => {
      const id = String(t.id);
      if (etabId) {
        await deleteTravelFromDb(etabId, id).catch((err) => {
          console.error(`[travels/list] purge ${id}:`, err);
        });
      }
    }),
  );

  const remaining = trips.filter((t) => !expiredIds.has(String(t.id)));
  await saveTravelsIndex(remaining);
  return remaining;
}

export async function GET() {
  const gate = await requireAuth();
  if (!gate.ok) return gate.response;
  try {
    const trips = await listTravelsIndex();
    const afterPurge = await purgeOldTrips(trips);
    const sortedTrips = [...afterPurge].sort(compareTripsByTravelDate).map(normalizeTripImageFields);
    return NextResponse.json(sortedTrips);
  } catch (error) {
    console.error("Erreur S3 List:", error);
    return NextResponse.json({ error: "Erreur lors de la récupération de l'index" }, { status: 500 });
  }
}
