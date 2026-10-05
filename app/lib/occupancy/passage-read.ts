import "server-only";

import { and, desc, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { passage } from "@/db/schema-socle-faits";
import type { OccupancySignal } from "./types";

/**
 * Dernier passage **portail** du jour civil.
 * `sortie` → hors établissement. `entree` → pas de signal (présumé dans les murs).
 * Self / internat / autre ne déplacent pas « où est X » ici.
 */
export async function listPassagePortailSignalsOnDate(opts: {
  etablissementId: string;
  /** YYYY-MM-DD */
  date: string;
  eleveIds?: string[];
}): Promise<OccupancySignal[]> {
  const db = getDb();
  const dayStart = `${opts.date}T00:00:00.000Z`;
  const dayEnd = `${opts.date}T23:59:59.999Z`;

  const conditions = [
    eq(passage.etablissementId, opts.etablissementId),
    eq(passage.lieu, "portail"),
    sql`${passage.eleveId} is not null`,
    sql`${passage.horodatage} >= ${dayStart}::timestamptz`,
    sql`${passage.horodatage} <= ${dayEnd}::timestamptz`,
  ];
  if (opts.eleveIds && opts.eleveIds.length > 0) {
    conditions.push(inArray(passage.eleveId, opts.eleveIds));
  }

  const rows = await db
    .select({
      id: passage.id,
      eleveId: passage.eleveId,
      sens: passage.sens,
      horodatage: passage.horodatage,
    })
    .from(passage)
    .where(and(...conditions))
    .orderBy(desc(passage.horodatage));

  const latestByEleve = new Map<
    string,
    { id: string; sens: string }
  >();
  for (const r of rows) {
    if (!r.eleveId) continue;
    if (latestByEleve.has(r.eleveId)) continue;
    latestByEleve.set(r.eleveId, { id: r.id, sens: r.sens });
  }

  const signals: OccupancySignal[] = [];
  for (const [eleveId, last] of latestByEleve) {
    if (last.sens !== "sortie") continue;
    signals.push({
      eleveId,
      tag: "hors_etablissement",
      source: `passage:${last.id}`,
      coverage: "complete",
      confidenceTag: "defined",
      detail: {
        passageId: last.id,
        lieu: "portail",
        sens: "sortie",
      },
    });
  }
  return signals;
}
