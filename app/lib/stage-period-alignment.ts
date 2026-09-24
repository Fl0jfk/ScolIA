import {
  getStagePeriodsForClass,
  type StageClassPeriod,
} from "@/app/lib/stage-periods-config";
import { formatPeriodRangeFr } from "@/app/lib/stage-schedule";
import type { StageConvention, StageSchedule } from "@/app/lib/stage-types";

export type StagePeriodAlignmentStatus =
  | "no_official_periods"
  | "aligned"
  | "outside";

export type StagePeriodAlignment = {
  status: StagePeriodAlignmentStatus;
  /** True quand une alerte grosse doit s’afficher (hors période). */
  outside: boolean;
  className: string;
  scheduleStart: string;
  scheduleEnd: string;
  scheduleLabel: string;
  officialPeriods: StageClassPeriod[];
  /** Période officielle liée (stagePeriodId) ou meilleure correspondance. */
  referencePeriod: StageClassPeriod | null;
  /** Jours du stage entièrement hors de toute période officielle. */
  daysFullyOutside: number;
  /** Jours du stage hors de la période de référence (si connue). */
  daysOutsideReference: number;
  message: string;
  shortMessage: string;
};

function isoDaysInclusive(start: string, end: string): string[] {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(start) || !/^\d{4}-\d{2}-\d{2}$/.test(end)) return [];
  if (end < start) return [];
  const out: string[] = [];
  const cur = new Date(`${start}T12:00:00`);
  const last = new Date(`${end}T12:00:00`);
  while (cur <= last) {
    const y = cur.getFullYear();
    const m = String(cur.getMonth() + 1).padStart(2, "0");
    const d = String(cur.getDate()).padStart(2, "0");
    out.push(`${y}-${m}-${d}`);
    cur.setDate(cur.getDate() + 1);
  }
  return out;
}

function containedInPeriod(start: string, end: string, period: StageClassPeriod): boolean {
  return start >= period.periodStart && end <= period.periodEnd;
}

function overlapDays(start: string, end: string, period: StageClassPeriod): number {
  const a = start > period.periodStart ? start : period.periodStart;
  const b = end < period.periodEnd ? end : period.periodEnd;
  return a <= b ? isoDaysInclusive(a, b).length : 0;
}

function pickReferencePeriod(
  periods: StageClassPeriod[],
  scheduleStart: string,
  scheduleEnd: string,
  stagePeriodId?: string,
): StageClassPeriod | null {
  if (!periods.length) return null;
  if (stagePeriodId) {
    const byId = periods.find((p) => p.id === stagePeriodId);
    if (byId) return byId;
  }
  let best: StageClassPeriod | null = null;
  let bestOverlap = -1;
  for (const p of periods) {
    const o = overlapDays(scheduleStart, scheduleEnd, p);
    if (o > bestOverlap) {
      bestOverlap = o;
      best = p;
    }
  }
  return best ?? periods[0] ?? null;
}

export function assessScheduleAgainstOfficialPeriods(params: {
  className: string;
  schedule: Pick<StageSchedule, "periodStart" | "periodEnd">;
  officialPeriods: StageClassPeriod[];
  stagePeriodId?: string;
  stageLabel?: string;
}): StagePeriodAlignment {
  const scheduleStart = String(params.schedule.periodStart || "").slice(0, 10);
  const scheduleEnd = String(params.schedule.periodEnd || "").slice(0, 10);
  const scheduleLabel =
    formatPeriodRangeFr(scheduleStart, scheduleEnd) ||
    (scheduleStart && scheduleEnd ? `${scheduleStart} → ${scheduleEnd}` : "—");
  const officialPeriods = params.officialPeriods;

  if (!officialPeriods.length) {
    return {
      status: "no_official_periods",
      outside: false,
      className: params.className,
      scheduleStart,
      scheduleEnd,
      scheduleLabel,
      officialPeriods: [],
      referencePeriod: null,
      daysFullyOutside: 0,
      daysOutsideReference: 0,
      message: "Aucune période officielle n’est configurée pour cette classe.",
      shortMessage: "Pas de période officielle",
    };
  }

  const stageDays = isoDaysInclusive(scheduleStart, scheduleEnd);
  const coveredByAny = new Set<string>();
  for (const p of officialPeriods) {
    for (const d of stageDays) {
      if (d >= p.periodStart && d <= p.periodEnd) coveredByAny.add(d);
    }
  }
  const daysFullyOutside = stageDays.filter((d) => !coveredByAny.has(d)).length;

  const referencePeriod = pickReferencePeriod(
    officialPeriods,
    scheduleStart,
    scheduleEnd,
    params.stagePeriodId,
  );

  let daysOutsideReference = 0;
  if (referencePeriod) {
    daysOutsideReference = stageDays.filter(
      (d) => d < referencePeriod.periodStart || d > referencePeriod.periodEnd,
    ).length;
  }

  const fullyAligned = officialPeriods.some((p) =>
    containedInPeriod(scheduleStart, scheduleEnd, p),
  );

  if (fullyAligned && daysFullyOutside === 0) {
    const matched =
      officialPeriods.find((p) => containedInPeriod(scheduleStart, scheduleEnd, p)) ||
      referencePeriod;
    return {
      status: "aligned",
      outside: false,
      className: params.className,
      scheduleStart,
      scheduleEnd,
      scheduleLabel,
      officialPeriods,
      referencePeriod: matched,
      daysFullyOutside: 0,
      daysOutsideReference: 0,
      message: matched
        ? `Dates dans la période officielle « ${matched.label} » (${formatPeriodRangeFr(matched.periodStart, matched.periodEnd) || `${matched.periodStart} → ${matched.periodEnd}`}).`
        : "Dates alignées sur une période officielle.",
      shortMessage: "Dans la période officielle",
    };
  }

  const officialList = officialPeriods
    .map(
      (p) =>
        `« ${p.label} » : ${formatPeriodRangeFr(p.periodStart, p.periodEnd) || `${p.periodStart} → ${p.periodEnd}`}`,
    )
    .join(" · ");

  const refLabel = referencePeriod
    ? `« ${referencePeriod.label} » (${formatPeriodRangeFr(referencePeriod.periodStart, referencePeriod.periodEnd) || `${referencePeriod.periodStart} → ${referencePeriod.periodEnd}`})`
    : null;

  const parts: string[] = [
    `Les dates du stage (${scheduleLabel}) ne correspondent pas aux périodes officielles de ${params.className || "la classe"}.`,
  ];
  if (refLabel && daysOutsideReference > 0) {
    parts.push(
      `${daysOutsideReference} jour${daysOutsideReference > 1 ? "s" : ""} hors de la période ${refLabel}.`,
    );
  } else if (daysFullyOutside > 0) {
    parts.push(
      `${daysFullyOutside} jour${daysFullyOutside > 1 ? "s" : ""} entièrement hors des périodes officielles.`,
    );
  }
  parts.push(`Périodes officielles : ${officialList}.`);

  return {
    status: "outside",
    outside: true,
    className: params.className,
    scheduleStart,
    scheduleEnd,
    scheduleLabel,
    officialPeriods,
    referencePeriod,
    daysFullyOutside,
    daysOutsideReference,
    message: parts.join(" "),
    shortMessage: "HORS PÉRIODE OFFICIELLE",
  };
}

export async function assessConventionPeriodAlignment(
  convention: Pick<StageConvention, "student" | "schedule" | "stagePeriodId" | "stageLabel" | "schoolYear">,
): Promise<StagePeriodAlignment> {
  const officialPeriods = await getStagePeriodsForClass(
    convention.student.className,
    convention.schoolYear,
  );
  return assessScheduleAgainstOfficialPeriods({
    className: convention.student.className,
    schedule: convention.schedule,
    officialPeriods,
    stagePeriodId: convention.stagePeriodId,
    stageLabel: convention.stageLabel,
  });
}

/** Propose un planning calé sur une période officielle (conserve jours / horaires). */
export function suggestScheduleForOfficialPeriod(
  schedule: StageSchedule,
  period: StageClassPeriod,
): StageSchedule {
  return {
    ...schedule,
    periodStart: period.periodStart,
    periodEnd: period.periodEnd,
  };
}
