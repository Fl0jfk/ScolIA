import "server-only";

import { and, eq, inArray, type SQL } from "drizzle-orm";
import type { getDb } from "@/db/index";
import { travelParticipant } from "@/db/schema";
import { chunkArray } from "@/app/lib/db-in-chunks";
import { isPgUndefinedColumnError } from "@/app/lib/travel-db-pg-errors";

const participantBaseSelect = {
  id: travelParticipant.id,
  etablissementId: travelParticipant.etablissementId,
  travelId: travelParticipant.travelId,
  eleveKey: travelParticipant.eleveKey,
  nom: travelParticipant.nom,
  prenom: travelParticipant.prenom,
  classe: travelParticipant.classe,
  droitImageOk: travelParticipant.droitImageOk,
  panierRepas: travelParticipant.panierRepas,
  sortOrder: travelParticipant.sortOrder,
};

export type TravelParticipantReadRow = {
  id: string;
  etablissementId: string;
  travelId: string;
  eleveKey: string;
  nom: string;
  prenom: string;
  classe: string | null;
  droitImageOk: boolean;
  panierRepas: boolean;
  sortOrder: number;
  eleveId?: string | null;
};

export async function queryTravelParticipants(
  db: ReturnType<typeof getDb>,
  where: SQL | undefined,
): Promise<TravelParticipantReadRow[]> {
  if (!where) return [];
  try {
    return await db
      .select({ ...participantBaseSelect, eleveId: travelParticipant.eleveId })
      .from(travelParticipant)
      .where(where);
  } catch (error) {
    if (!isPgUndefinedColumnError(error, "eleve_id")) throw error;
    console.warn(
      "[travel-db] colonne travel_participant.eleve_id absente — appliquer drizzle/0051_travel_participant_eleve_id.sql ; lecture liste sans FK",
    );
    return await db.select(participantBaseSelect).from(travelParticipant).where(where);
  }
}

export async function listTravelParticipantsForTripIds(
  db: ReturnType<typeof getDb>,
  etablissementId: string,
  travelIds: string[],
): Promise<TravelParticipantReadRow[]> {
  const ids = [...new Set(travelIds.filter(Boolean))];
  if (ids.length === 0) return [];
  const out: TravelParticipantReadRow[] = [];
  for (const batch of chunkArray(ids)) {
    const rows = await queryTravelParticipants(
      db,
      and(
        eq(travelParticipant.etablissementId, etablissementId),
        inArray(travelParticipant.travelId, batch),
      ),
    );
    out.push(...rows);
  }
  return out;
}
