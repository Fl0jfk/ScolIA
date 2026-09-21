import "server-only";

import { and, eq, inArray, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve, eleveScolarite } from "@/db/schema";
import { listAbsenceSignalsOnDate } from "./absences-read";
import { factsByEleveId, mergeCoverage, mergeOccupancySignals } from "./merge";
import { listTravelPresenceOnDate, travelRowsToSignals } from "./travels-read";
import type { OccupancyFact, OccupancyResult, OccupancySignal } from "./types";

export type OccupancyQuery = {
  etablissementId: string;
  /** YYYY-MM-DD */
  date: string;
  eleveId?: string;
  /** Classe live (scolarité en_cours, repli plat). */
  classe?: string;
  eleveIds?: string[];
};

async function resolveTargetEleveIds(opts: OccupancyQuery): Promise<{
  eleveIds: string[];
  liveClasseByEleve: Map<string, string | null>;
  coverageNote: "complete" | "partial";
}> {
  const db = getDb();
  const liveClasseByEleve = new Map<string, string | null>();

  if (opts.eleveId) {
    const [row] = await db
      .select({
        id: eleve.id,
        classePlat: eleve.classe,
      })
      .from(eleve)
      .where(and(eq(eleve.etablissementId, opts.etablissementId), eq(eleve.id, opts.eleveId)))
      .limit(1);
    if (!row) return { eleveIds: [], liveClasseByEleve, coverageNote: "partial" };

    const [scol] = await db
      .select({ classe: eleveScolarite.classe })
      .from(eleveScolarite)
      .where(
        and(
          eq(eleveScolarite.etablissementId, opts.etablissementId),
          eq(eleveScolarite.eleveId, opts.eleveId),
          eq(eleveScolarite.statut, "en_cours"),
        ),
      )
      .limit(1);
    liveClasseByEleve.set(row.id, scol?.classe ?? row.classePlat);
    return { eleveIds: [row.id], liveClasseByEleve, coverageNote: "complete" };
  }

  if (opts.eleveIds && opts.eleveIds.length > 0) {
    const ids = [...new Set(opts.eleveIds)];
    const rows = await db
      .select({
        id: eleve.id,
        classePlat: eleve.classe,
        classeScol: eleveScolarite.classe,
      })
      .from(eleve)
      .leftJoin(
        eleveScolarite,
        and(
          eq(eleveScolarite.eleveId, eleve.id),
          eq(eleveScolarite.etablissementId, eleve.etablissementId),
          eq(eleveScolarite.statut, "en_cours"),
        ),
      )
      .where(and(eq(eleve.etablissementId, opts.etablissementId), inArray(eleve.id, ids)));
    for (const r of rows) {
      liveClasseByEleve.set(r.id, r.classeScol ?? r.classePlat);
    }
    return { eleveIds: rows.map((r) => r.id), liveClasseByEleve, coverageNote: "complete" };
  }

  if (opts.classe?.trim()) {
    const classe = opts.classe.trim();
    const fromScol = await db
      .select({
        id: eleve.id,
        classeScol: eleveScolarite.classe,
        classePlat: eleve.classe,
      })
      .from(eleve)
      .innerJoin(
        eleveScolarite,
        and(
          eq(eleveScolarite.eleveId, eleve.id),
          eq(eleveScolarite.etablissementId, eleve.etablissementId),
          eq(eleveScolarite.statut, "en_cours"),
        ),
      )
      .where(
        and(eq(eleve.etablissementId, opts.etablissementId), eq(eleveScolarite.classe, classe)),
      );

    const fromPlat = await db
      .select({
        id: eleve.id,
        classePlat: eleve.classe,
      })
      .from(eleve)
      .where(
        and(
          eq(eleve.etablissementId, opts.etablissementId),
          eq(eleve.classe, classe),
          sql`NOT EXISTS (
            SELECT 1 FROM eleve_scolarite s
            WHERE s.etablissement_id = ${opts.etablissementId}
              AND s.eleve_id = ${eleve.id}
              AND s.statut = 'en_cours'
          )`,
        ),
      );

    for (const r of fromScol) {
      liveClasseByEleve.set(r.id, r.classeScol ?? r.classePlat);
    }
    for (const r of fromPlat) {
      if (!liveClasseByEleve.has(r.id)) liveClasseByEleve.set(r.id, r.classePlat);
    }
    return {
      eleveIds: [...liveClasseByEleve.keys()],
      liveClasseByEleve,
      coverageNote: "complete",
    };
  }

  return { eleveIds: [], liveClasseByEleve, coverageNote: "partial" };
}

/**
 * Lecture unique de présence pour UI / famille filtrée / IA.
 * **Zéro** INSERT `vs_absence_eleve`. Internat nominatif = hors lot (coverage partial).
 */
export async function occupancy(opts: OccupancyQuery): Promise<OccupancyResult> {
  const { eleveIds, liveClasseByEleve, coverageNote } = await resolveTargetEleveIds(opts);

  if (eleveIds.length === 0 && !opts.classe && !opts.eleveId && !opts.eleveIds) {
    return {
      etablissementId: opts.etablissementId,
      date: opts.date,
      facts: [],
      unmatchedParticipants: [],
      coverage: "partial",
    };
  }

  const filterIds = eleveIds.length > 0 ? eleveIds : undefined;

  const [travelPresence, absenceSignals] = await Promise.all([
    listTravelPresenceOnDate({
      etablissementId: opts.etablissementId,
      date: opts.date,
      eleveIds: filterIds,
    }),
    listAbsenceSignalsOnDate({
      etablissementId: opts.etablissementId,
      date: opts.date,
      eleveIds: filterIds,
    }),
  ]);

  const travelSignals = travelRowsToSignals(travelPresence.rows, liveClasseByEleve);
  const allSignals: OccupancySignal[] = [...travelSignals, ...absenceSignals];

  const targetIds =
    eleveIds.length > 0
      ? eleveIds
      : [...new Set(allSignals.map((s) => s.eleveId))];

  const facts: OccupancyFact[] = targetIds.map((id) =>
    mergeOccupancySignals({
      eleveId: id,
      date: opts.date,
      signals: allSignals.filter((s) => s.eleveId === id),
      defaultTag: "en_cours",
    }),
  );

  const coverages = [
    coverageNote,
    travelPresence.unmatched.length > 0 ? ("partial" as const) : ("complete" as const),
    // Internat nominatif non branché → partial assumé si on élargit plus tard.
  ];

  return {
    etablissementId: opts.etablissementId,
    date: opts.date,
    facts,
    unmatchedParticipants: travelPresence.unmatched,
    coverage: mergeCoverage(coverages),
  };
}

/** Capability `get_presence_jour` — même contrat que `occupancy`. */
export async function getPresenceJour(opts: OccupancyQuery): Promise<OccupancyResult> {
  return occupancy(opts);
}

export async function occupancyFactForEleve(opts: {
  etablissementId: string;
  date: string;
  eleveId: string;
}): Promise<OccupancyFact | null> {
  const result = await occupancy({
    etablissementId: opts.etablissementId,
    date: opts.date,
    eleveId: opts.eleveId,
  });
  return result.facts[0] ?? null;
}

export { factsByEleveId };
