/**
 * Grilles horaires établissement (sonneries / périodes).
 * Plusieurs grilles possibles (école, collège, lycée…) — paramétrables.
 */

export type TimetablePeriodKind = "lesson" | "break" | "lunch";

export type TimetablePeriod = {
  id: string;
  start: string;
  end: string;
  kind: TimetablePeriodKind;
  label?: string;
};

export type TimetableGrid = {
  id: string;
  label: string;
  /** Site / établissement (école, collège…) si multi-sites. */
  establishmentId?: string | null;
  kind?: "ecole" | "college" | "lycee" | "custom";
  periods: TimetablePeriod[];
};

export type TimetableGridsConfig = {
  grids: TimetableGrid[];
  defaultGridId?: string;
};

function hhmmOk(v: string): boolean {
  return /^([01]\d|2[0-3]):[0-5]\d$/.test(v);
}

function toMin(hhmm: string): number {
  const [h, m] = hhmm.split(":").map(Number);
  return (h || 0) * 60 + (m || 0);
}

function newId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

/** Grille type Providence (collège/lycée) — matin officiel + après-midi éditable. */
export function defaultProvidenceCollegeGrid(): TimetableGrid {
  return {
    id: "college-defaut",
    label: "Collège / lycée",
    kind: "college",
    periods: [
      { id: "p1", start: "08:30", end: "09:25", kind: "lesson", label: "1re heure" },
      { id: "p2", start: "09:25", end: "10:20", kind: "lesson", label: "2e heure" },
      { id: "pause-am", start: "10:20", end: "10:35", kind: "break", label: "Récréation" },
      { id: "p3", start: "10:35", end: "11:30", kind: "lesson", label: "3e heure" },
      { id: "p4", start: "11:30", end: "12:25", kind: "lesson", label: "4e heure" },
      { id: "midi", start: "12:25", end: "13:30", kind: "lunch", label: "Pause méridienne" },
      { id: "p5", start: "13:30", end: "14:25", kind: "lesson", label: "5e heure" },
      { id: "p6", start: "14:25", end: "15:20", kind: "lesson", label: "6e heure" },
      { id: "pause-pm", start: "15:20", end: "15:35", kind: "break", label: "Récréation" },
      { id: "p7", start: "15:35", end: "16:30", kind: "lesson", label: "7e heure" },
      { id: "p8", start: "16:30", end: "17:25", kind: "lesson", label: "8e heure" },
    ],
  };
}

export function defaultTimetableGridsConfig(): TimetableGridsConfig {
  const grid = defaultProvidenceCollegeGrid();
  return { grids: [grid], defaultGridId: grid.id };
}

export function parseTimetablePeriod(raw: unknown): TimetablePeriod {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const start = typeof o.start === "string" && hhmmOk(o.start) ? o.start : "08:00";
  const end = typeof o.end === "string" && hhmmOk(o.end) ? o.end : "09:00";
  const kindRaw = typeof o.kind === "string" ? o.kind : "lesson";
  const kind: TimetablePeriodKind =
    kindRaw === "break" || kindRaw === "lunch" || kindRaw === "lesson" ? kindRaw : "lesson";
  const id = typeof o.id === "string" && o.id.trim() ? o.id.trim() : newId("period");
  const label = typeof o.label === "string" && o.label.trim() ? o.label.trim() : undefined;
  if (toMin(start) >= toMin(end)) {
    throw new Error(`Période invalide (${start} → ${end}).`);
  }
  return { id, start, end, kind, label };
}

export function parseTimetableGrid(raw: unknown): TimetableGrid {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const id = typeof o.id === "string" && o.id.trim() ? o.id.trim() : newId("grid");
  const label = typeof o.label === "string" && o.label.trim() ? o.label.trim() : id;
  const establishmentId =
    typeof o.establishmentId === "string" && o.establishmentId.trim()
      ? o.establishmentId.trim()
      : null;
  const kindRaw = typeof o.kind === "string" ? o.kind : "custom";
  const kind =
    kindRaw === "ecole" || kindRaw === "college" || kindRaw === "lycee" || kindRaw === "custom"
      ? kindRaw
      : "custom";
  const periods = Array.isArray(o.periods)
    ? o.periods.map(parseTimetablePeriod).sort((a, b) => a.start.localeCompare(b.start))
    : [];
  return { id, label, establishmentId, kind, periods };
}

export function parseTimetableGridsConfig(raw: unknown): TimetableGridsConfig {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const grids = Array.isArray(o.grids) ? o.grids.map(parseTimetableGrid) : [];
  if (grids.length === 0) return defaultTimetableGridsConfig();
  const defaultGridId =
    typeof o.defaultGridId === "string" && grids.some((g) => g.id === o.defaultGridId)
      ? o.defaultGridId
      : grids[0]?.id;
  return { grids, defaultGridId };
}

export function lessonPeriods(grid: TimetableGrid): TimetablePeriod[] {
  return grid.periods
    .filter((p) => p.kind === "lesson")
    .sort((a, b) => a.start.localeCompare(b.start));
}

export function lessonStartTimes(grid: TimetableGrid): string[] {
  return lessonPeriods(grid).map((p) => p.start);
}

/**
 * Calcule l’heure de fin pour un début de cours + N heures de cours,
 * en enjambant pauses / midi (ils ne comptent pas dans N).
 */
export function resolveSlotRangeFromDuration(
  grid: TimetableGrid,
  startHhmm: string,
  lessonCount: number,
): { start: string; end: string; lessonCount: number } | null {
  const lessons = lessonPeriods(grid);
  if (lessons.length === 0) return null;
  const n = Math.max(1, Math.min(8, Math.floor(lessonCount) || 1));

  let idx = lessons.findIndex((p) => p.start === startHhmm);
  if (idx < 0) {
    const startMin = toMin(startHhmm);
    idx = lessons.findIndex((p) => toMin(p.start) >= startMin);
    if (idx < 0) idx = lessons.length - 1;
  }

  const lastIdx = Math.min(lessons.length - 1, idx + n - 1);
  const start = lessons[idx]!.start;
  const end = lessons[lastIdx]!.end;
  return { start, end, lessonCount: lastIdx - idx + 1 };
}

/** Estime combien d’heures de cours couvrent [start, end] sur la grille. */
export function inferLessonCount(
  grid: TimetableGrid,
  startHhmm: string,
  endHhmm: string,
): number {
  const lessons = lessonPeriods(grid);
  if (lessons.length === 0) return 1;
  const s = toMin(startHhmm);
  const e = toMin(endHhmm);
  const covered = lessons.filter((p) => toMin(p.start) >= s - 1 && toMin(p.end) <= e + 1);
  if (covered.length > 0) return covered.length;
  const overlapping = lessons.filter(
    (p) => toMin(p.start) < e && toMin(p.end) > s,
  );
  return Math.max(1, overlapping.length);
}

/** Snap une heure libre vers le début de cours le plus proche (avant ou égal). */
export function snapToLessonStart(grid: TimetableGrid, hhmm: string): string {
  const lessons = lessonPeriods(grid);
  if (lessons.length === 0) return hhmm;
  const target = toMin(hhmm);
  let best = lessons[0]!.start;
  let bestDist = Math.abs(toMin(best) - target);
  for (const p of lessons) {
    const d = Math.abs(toMin(p.start) - target);
    if (d < bestDist) {
      best = p.start;
      bestDist = d;
    }
  }
  return best;
}

export function pickDefaultTimetableGrid(
  config: TimetableGridsConfig,
  establishmentId?: string | null,
): TimetableGrid | null {
  if (!config.grids.length) return null;
  if (establishmentId) {
    const bySite = config.grids.find((g) => g.establishmentId === establishmentId);
    if (bySite) return bySite;
  }
  const byDefault = config.grids.find((g) => g.id === config.defaultGridId);
  return byDefault || config.grids[0] || null;
}

export function emptyTimetablePeriod(
  kind: TimetablePeriodKind = "lesson",
  start = "08:30",
  end = "09:25",
): TimetablePeriod {
  return {
    id: newId("period"),
    start,
    end,
    kind,
    label: kind === "lesson" ? "Cours" : kind === "lunch" ? "Pause midi" : "Pause",
  };
}

export function emptyTimetableGrid(partial?: Partial<TimetableGrid>): TimetableGrid {
  return {
    id: newId("grid"),
    label: "Nouvelle grille",
    kind: "custom",
    establishmentId: null,
    periods: defaultProvidenceCollegeGrid().periods.map((p) => ({
      ...p,
      id: newId("period"),
    })),
    ...partial,
  };
}
