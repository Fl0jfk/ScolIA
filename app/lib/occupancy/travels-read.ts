import "server-only";

import { and, eq, gte, inArray, isNotNull, lte, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { travel, travelParticipant } from "@/db/schema";
import type { OccupancySignal, UnmatchedTravelParticipant } from "./types";

/** Statuts dossier pour lesquels la participation compte en occupancy. */
export const OCCUPANCY_TRAVEL_STATUSES = ["VALIDE", "FINALISE_DIR_ATTENTE_ELEVES"] as const;

export type TravelPresenceRow = {
  travelId: string;
  eleveId: string | null;
  eleveKey: string;
  nom: string;
  prenom: string;
  snapshotClasse: string | null;
  status: string;
  startDate: string | null;
  endDate: string | null;
};

/**
 * Participants de sorties actives dont la date tombe dans [startDate, endDate].
 * Ne lit pas la classe snapshot comme effectif — seulement pour détail document.
 * Les lignes sans `eleve_id` sont toujours remontées en `unmatched` (revue),
 * même si on filtre par ids élèves.
 */
export async function listTravelPresenceOnDate(opts: {
  etablissementId: string;
  date: string;
  eleveIds?: string[];
}): Promise<{ rows: TravelPresenceRow[]; unmatched: UnmatchedTravelParticipant[] }> {
  const db = getDb();
  const baseConditions = [
    eq(travel.etablissementId, opts.etablissementId),
    eq(travelParticipant.etablissementId, opts.etablissementId),
    inArray(travel.status, [...OCCUPANCY_TRAVEL_STATUSES]),
    lte(travel.startDate, opts.date),
    gte(travel.endDate, opts.date),
  ];

  const allOnDate = await db
    .select({
      travelId: travel.id,
      eleveId: travelParticipant.eleveId,
      eleveKey: travelParticipant.eleveKey,
      nom: travelParticipant.nom,
      prenom: travelParticipant.prenom,
      snapshotClasse: travelParticipant.classe,
      status: travel.status,
      startDate: travel.startDate,
      endDate: travel.endDate,
    })
    .from(travelParticipant)
    .innerJoin(
      travel,
      and(
        eq(travel.id, travelParticipant.travelId),
        eq(travel.etablissementId, travelParticipant.etablissementId),
      ),
    )
    .where(and(...baseConditions));

  const unmatched: UnmatchedTravelParticipant[] = [];
  const linked: TravelPresenceRow[] = [];
  const filter = opts.eleveIds && opts.eleveIds.length > 0 ? new Set(opts.eleveIds) : null;

  for (const row of allOnDate) {
    if (!row.eleveId) {
      unmatched.push({
        travelId: row.travelId,
        eleveKey: row.eleveKey,
        nom: row.nom,
        prenom: row.prenom,
        snapshotClasse: row.snapshotClasse,
      });
      continue;
    }
    if (filter && !filter.has(row.eleveId)) continue;
    linked.push(row);
  }

  return { rows: linked, unmatched };
}

export function travelRowsToSignals(
  rows: TravelPresenceRow[],
  liveClasseByEleve?: Map<string, string | null>,
): OccupancySignal[] {
  return rows
    .filter((r): r is TravelPresenceRow & { eleveId: string } => Boolean(r.eleveId))
    .map((r) => ({
      eleveId: r.eleveId,
      tag: "en_sortie" as const,
      source: `travel:${r.travelId}`,
      coverage: "complete" as const,
      confidenceTag: "defined" as const,
      detail: {
        travelId: r.travelId,
        snapshotClasse: r.snapshotClasse,
        liveClasse: liveClasseByEleve?.get(r.eleveId) ?? null,
      },
    }));
}

/** Ids élèves en sortie un jour donné (pour exclure du bulletin à l'appel). */
export async function listEleveIdsEnSortieOnDate(opts: {
  etablissementId: string;
  date: string;
  eleveIds?: string[];
}): Promise<Set<string>> {
  const { rows } = await listTravelPresenceOnDate(opts);
  const set = new Set<string>();
  for (const r of rows) {
    if (r.eleveId) set.add(r.eleveId);
  }
  return set;
}

/** Relit uniquement les participants déjà liés (index). */
export async function countLinkedParticipants(etablissementId: string): Promise<number> {
  const db = getDb();
  const [row] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(travelParticipant)
    .where(
      and(
        eq(travelParticipant.etablissementId, etablissementId),
        isNotNull(travelParticipant.eleveId),
      ),
    );
  return row?.n ?? 0;
}
