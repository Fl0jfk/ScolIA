import "server-only";

import { and, eq, inArray, isNull, lte, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { infirmeriePassage } from "@/db/schema-socle-faits";
import type { OccupancySignal } from "./types";

/**
 * Élèves encore à l’infirmerie le jour donné (arrivée ≤ fin de journée, sortie nulle).
 * Le motif court n’alimente pas le dossier médical — signal VS seulement.
 */
export async function listInfirmerieSignalsOnDate(opts: {
  etablissementId: string;
  /** YYYY-MM-DD */
  date: string;
  eleveIds?: string[];
}): Promise<OccupancySignal[]> {
  const db = getDb();
  const dayEnd = `${opts.date}T23:59:59.999Z`;

  const conditions = [
    eq(infirmeriePassage.etablissementId, opts.etablissementId),
    eq(infirmeriePassage.signalVieScolaire, true),
    isNull(infirmeriePassage.sortie),
    lte(infirmeriePassage.arrivee, sql`${dayEnd}::timestamptz`),
  ];
  if (opts.eleveIds && opts.eleveIds.length > 0) {
    conditions.push(inArray(infirmeriePassage.eleveId, opts.eleveIds));
  }

  const rows = await db
    .select({
      id: infirmeriePassage.id,
      eleveId: infirmeriePassage.eleveId,
      motifCourt: infirmeriePassage.motifCourt,
    })
    .from(infirmeriePassage)
    .where(and(...conditions));

  return rows.map((r) => ({
    eleveId: r.eleveId,
    tag: "a_infirmerie" as const,
    source: `infirmerie_passage:${r.id}`,
    coverage: "complete" as const,
    confidenceTag: "defined" as const,
    detail: {
      infirmeriePassageId: r.id,
      motif: r.motifCourt || null,
      lieu: "infirmerie",
    },
  }));
}
