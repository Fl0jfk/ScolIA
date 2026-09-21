import "server-only";

import { and, eq, inArray, lte, gte } from "drizzle-orm";
import { getDb } from "@/db/index";
import { vsAbsenceEleve } from "@/db/schema";
import { isStageAbsenceMotif, type OccupancySignal } from "./types";

export type AbsencePresenceRow = {
  id: string;
  eleveId: string;
  motif: string | null;
  source: string;
  type: string;
  dateDebut: string;
  dateFin: string;
};

/**
 * Absences VS couvrant la date (bulletin / accueil / famille…).
 * Les motifs « Stage — … » sont étiquetés `en_stage`, pas `absent_vs`.
 */
export async function listAbsenceSignalsOnDate(opts: {
  etablissementId: string;
  date: string;
  eleveIds?: string[];
}): Promise<OccupancySignal[]> {
  const db = getDb();
  const conditions = [
    eq(vsAbsenceEleve.etablissementId, opts.etablissementId),
    lte(vsAbsenceEleve.dateDebut, opts.date),
    gte(vsAbsenceEleve.dateFin, opts.date),
  ];
  if (opts.eleveIds && opts.eleveIds.length > 0) {
    conditions.push(inArray(vsAbsenceEleve.eleveId, opts.eleveIds));
  }

  const rows = await db
    .select({
      id: vsAbsenceEleve.id,
      eleveId: vsAbsenceEleve.eleveId,
      motif: vsAbsenceEleve.motif,
      source: vsAbsenceEleve.source,
      type: vsAbsenceEleve.type,
      dateDebut: vsAbsenceEleve.dateDebut,
      dateFin: vsAbsenceEleve.dateFin,
    })
    .from(vsAbsenceEleve)
    .where(and(...conditions));

  return rows.map((r) => {
    const stage = isStageAbsenceMotif(r.motif);
    return {
      eleveId: r.eleveId,
      tag: stage ? ("en_stage" as const) : ("absent_vs" as const),
      source: `vs_absence:${r.id}`,
      coverage: "complete" as const,
      confidenceTag: "defined" as const,
      detail: {
        absenceId: r.id,
        motif: r.motif,
      },
    };
  });
}
