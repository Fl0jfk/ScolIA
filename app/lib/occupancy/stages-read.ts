import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { eleve } from "@/db/schema";
import { getConventionsIndex, getStageConvention } from "@/app/lib/stage-storage";
import type { StageConvention, StageConventionStatus } from "@/app/lib/stage-types";
import type { OccupancySignal } from "./types";

/** Statuts pour lesquels l’élève est « en stage » (ailleurs, hors bulletin). */
export const OCCUPANCY_STAGE_STATUSES: readonly StageConventionStatus[] = [
  "signed",
  "signatures_pending",
  "convention_ready",
  "convention_deposited",
] as const;

export type StagePresenceRow = {
  conventionId: string;
  eleveId: string;
  companyName: string;
  periodStart: string;
  periodEnd: string;
  status: StageConventionStatus;
  studentName: string;
  className: string;
};

function coversDate(start: string, end: string, date: string): boolean {
  const s = start.slice(0, 10);
  const e = end.slice(0, 10);
  const d = date.slice(0, 10);
  if (!s || !e || !d) return false;
  return s <= d && d <= e;
}

async function resolveEleveIdFromConvention(
  etablissementId: string,
  convention: StageConvention,
): Promise<string | null> {
  const filed = convention.eleveDossierFiling?.eleveId?.trim();
  if (filed) return filed;

  const db = getDb();
  const ine = convention.ocrMeta?.matchedEleveIne?.trim().toUpperCase() || "";
  if (ine) {
    const [byIne] = await db
      .select({ id: eleve.id })
      .from(eleve)
      .where(and(eq(eleve.etablissementId, etablissementId), eq(eleve.ine, ine)))
      .limit(1);
    if (byIne) return byIne.id;
  }

  const nom = convention.student.lastName?.trim();
  const prenom = convention.student.firstName?.trim();
  if (!nom || !prenom) return null;
  const candidates = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        eq(eleve.nom, nom),
        eq(eleve.prenom, prenom),
      ),
    )
    .limit(5);
  if (candidates.length === 1) return candidates[0]!.id;

  const upper = await db
    .select({ id: eleve.id })
    .from(eleve)
    .where(
      and(
        eq(eleve.etablissementId, etablissementId),
        eq(eleve.nom, nom.toUpperCase()),
        eq(eleve.prenom, prenom),
      ),
    )
    .limit(5);
  if (upper.length === 1) return upper[0]!.id;
  return null;
}

/**
 * Présence stage live depuis les conventions (SoT période), comme les voyages.
 * Ne lit / n’écrit pas `vs_absence_eleve`.
 */
export async function listStagePresenceOnDate(opts: {
  etablissementId: string;
  date: string;
  eleveIds?: string[];
}): Promise<{ rows: StagePresenceRow[]; unmatched: number }> {
  const index = await getConventionsIndex();
  const filter = opts.eleveIds && opts.eleveIds.length > 0 ? new Set(opts.eleveIds) : null;
  const rows: StagePresenceRow[] = [];
  let unmatched = 0;

  const candidates = index.filter(
    (e) =>
      (OCCUPANCY_STAGE_STATUSES as readonly string[]).includes(e.status) &&
      coversDate(e.periodStart, e.periodEnd, opts.date),
  );

  for (const entry of candidates) {
    const convention = await getStageConvention(entry.id);
    if (!convention) continue;
    if (!(OCCUPANCY_STAGE_STATUSES as readonly string[]).includes(convention.status)) continue;
    const start = convention.schedule.periodStart?.slice(0, 10) || entry.periodStart;
    const end = convention.schedule.periodEnd?.slice(0, 10) || entry.periodEnd;
    if (!coversDate(start, end, opts.date)) continue;

    const eleveId = await resolveEleveIdFromConvention(opts.etablissementId, convention);
    if (!eleveId) {
      unmatched += 1;
      continue;
    }
    if (filter && !filter.has(eleveId)) continue;

    rows.push({
      conventionId: convention.id,
      eleveId,
      companyName: convention.company.name || entry.companyName,
      periodStart: start,
      periodEnd: end,
      status: convention.status,
      studentName:
        `${convention.student.firstName} ${convention.student.lastName}`.trim() ||
        entry.studentName,
      className: convention.student.className || entry.className,
    });
  }

  return { rows, unmatched };
}

export function stageRowsToSignals(rows: StagePresenceRow[]): OccupancySignal[] {
  return rows.map((r) => ({
    eleveId: r.eleveId,
    tag: "en_stage" as const,
    source: `stage:${r.conventionId}`,
    coverage: "complete" as const,
    confidenceTag: "defined" as const,
    detail: {
      motif: `Stage — ${r.companyName}`,
      travelId: undefined,
    },
  }));
}

export async function listEleveIdsEnStageOnDate(opts: {
  etablissementId: string;
  date: string;
  eleveIds?: string[];
}): Promise<Set<string>> {
  const { rows } = await listStagePresenceOnDate(opts);
  return new Set(rows.map((r) => r.eleveId));
}
