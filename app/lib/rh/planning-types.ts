/** Planning RH — professeurs (semaines A/B) et personnels OGEC (fixe / missions). */

export type PlanningWeekday = 1 | 2 | 3 | 4 | 5; // lun–ven

export const PLANNING_WEEKDAY_LABELS: Record<PlanningWeekday, string> = {
  1: "Lundi",
  2: "Mardi",
  3: "Mercredi",
  4: "Jeudi",
  5: "Vendredi",
};

export const PLANNING_WEEKDAYS = [1, 2, 3, 4, 5] as const satisfies readonly PlanningWeekday[];

/** Lieux types pour missions / surveillance. */
export const SURVEILLANT_LOCATION_SUGGESTIONS = [
  "Entrée établissement",
  "Hall",
  "Cour",
  "Étude",
  "Internat",
  "Réfectoire",
  "Couloir",
  "CDI",
] as const;

/**
 * Nombre de semaines travaillées utilisées pour estimer le volume annuel
 * à partir d’une semaine type (hors vacances / RTT — approximation RH).
 */
export const PLANNING_ANNUAL_WEEKS_FACTOR = 36;

export type PlanningTimeSlot = {
  id: string;
  day: PlanningWeekday;
  start: string; // HH:MM
  end: string;
};

export type TeacherPlanningSlot = PlanningTimeSlot & {
  subject: string;
  classes: string[];
  room?: string;
};

/** Créneau de remplacement daté (en plus des semaines types A/B). */
export type TeacherReplacementSlot = {
  id: string;
  date: string; // YYYY-MM-DD
  start: string;
  end: string;
  subject: string;
  classes: string[];
  room?: string;
  note?: string;
  createdBy: string;
  createdAt: string;
};

export type TeacherPlanningDoc = {
  version: 1;
  kind: "teacher";
  personnelId: string;
  /** Semaine type A — valable toute l’année scolaire. */
  weekA: TeacherPlanningSlot[];
  /** Semaine type B — valable toute l’année scolaire. */
  weekB: TeacherPlanningSlot[];
  replacements: TeacherReplacementSlot[];
  updatedAt: string;
  updatedBy: string;
  source?: "manual" | "pdf_import";
  sourceFileName?: string;
};

/** Poste fixe (admin / compta / maintenance). */
export type StaffFixedSlot = PlanningTimeSlot & {
  label: string;
};

/** Mission / lieu (éducation, surveillants…). */
export type StaffMissionSlot = PlanningTimeSlot & {
  mission: string;
  location?: string;
};

export type StaffRotationWeek = {
  id: string;
  label: string;
  startDate?: string | null;
  endDate?: string | null;
  slots: StaffMissionSlot[];
};

export type PlanningExceptionKind = "avance" | "ajustement" | "depassement";

/** Exception sur un jour calendaire (horaire réel vs semaine type). */
export type PlanningDayException = {
  id: string;
  date: string; // YYYY-MM-DD
  start: string;
  end: string;
  kind: PlanningExceptionKind;
  note?: string;
  createdBy: string;
  createdAt: string;
};

export type StaffPlanningDoc = {
  version: 1;
  kind: "staff";
  personnelId: string;
  mode: "fixed" | "rotation";
  fixedSlots: StaffFixedSlot[];
  rotations: StaffRotationWeek[];
  /** Quota d’heures annuel (admin / compta / maintenance). */
  annualHoursTarget?: number;
  /** Ajustements jour (avance sur quota, etc.). */
  exceptions: PlanningDayException[];
  updatedAt: string;
  updatedBy: string;
  source?: "manual" | "pdf_import";
  sourceFileName?: string;
};

export type RhPlanningDoc = TeacherPlanningDoc | StaffPlanningDoc;
export type RhPlanningKind = RhPlanningDoc["kind"];

export type AnnualBalanceEstimate = {
  weeklyHours: number;
  projectedAnnualHours: number;
  annualHoursTarget: number | null;
  exceptionDeltaHours: number;
  /** projected − target + deltas (négatif = reste à faire). */
  balanceHours: number | null;
  weeksFactor: number;
};

export function teacherPlanningKey(personnelId: string) {
  return `rh/planning/teachers/${personnelId}.json`;
}

export function staffPlanningKey(personnelId: string) {
  return `rh/planning/staff/${personnelId}.json`;
}

export function planningKeyFor(kind: RhPlanningKind, personnelId: string) {
  return kind === "teacher" ? teacherPlanningKey(personnelId) : staffPlanningKey(personnelId);
}

const TIME_RE = /^([01]\d|2[0-3]):[0-5]\d$/;
const DATE_RE = /^\d{4}-\d{2}-\d{2}$/;

export function isPlanningWeekday(v: unknown): v is PlanningWeekday {
  return v === 1 || v === 2 || v === 3 || v === 4 || v === 5;
}

export function isValidPlanningTime(v: unknown): v is string {
  return typeof v === "string" && TIME_RE.test(v);
}

export function isValidIsoDate(v: unknown): v is string {
  return typeof v === "string" && DATE_RE.test(v);
}

function str(v: unknown, max = 200) {
  return typeof v === "string" ? v.trim().slice(0, max) : "";
}

function uid(prefix: string) {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

/** Minutes entre deux HH:MM (peut être négatif si end < start). */
export function planningTimeToMinutes(t: string): number {
  const [h, m] = t.split(":").map(Number);
  return (h || 0) * 60 + (m || 0);
}

export function planningSlotHours(start: string, end: string): number {
  const delta = planningTimeToMinutes(end) - planningTimeToMinutes(start);
  return delta > 0 ? delta / 60 : 0;
}

function weekdayFromIsoDate(date: string): PlanningWeekday | null {
  if (!isValidIsoDate(date)) return null;
  const d = new Date(`${date}T12:00:00`);
  if (Number.isNaN(d.getTime())) return null;
  const js = d.getDay(); // 0=dim … 6=sam
  if (js === 0 || js === 6) return null;
  return js as PlanningWeekday;
}

function normalizeTimeSlotBase(raw: unknown): PlanningTimeSlot | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  if (!isPlanningWeekday(o.day) || !isValidPlanningTime(o.start) || !isValidPlanningTime(o.end)) {
    return null;
  }
  if (o.start >= o.end) return null;
  return {
    id: str(o.id, 64) || uid("slot"),
    day: o.day,
    start: o.start,
    end: o.end,
  };
}

export function normalizeTeacherSlot(raw: unknown): TeacherPlanningSlot | null {
  const base = normalizeTimeSlotBase(raw);
  if (!base) return null;
  const o = raw as Record<string, unknown>;
  const subject = str(o.subject, 120);
  if (!subject) return null;
  const classes = Array.isArray(o.classes)
    ? o.classes.map((c) => str(c, 40)).filter(Boolean).slice(0, 12)
    : [];
  const room = str(o.room, 40) || undefined;
  return { ...base, subject, classes, room };
}

export function normalizeStaffFixedSlot(raw: unknown): StaffFixedSlot | null {
  const base = normalizeTimeSlotBase(raw);
  if (!base) return null;
  const label = str((raw as Record<string, unknown>).label, 120);
  if (!label) return null;
  return { ...base, label };
}

export function normalizeStaffMissionSlot(raw: unknown): StaffMissionSlot | null {
  const base = normalizeTimeSlotBase(raw);
  if (!base) return null;
  const o = raw as Record<string, unknown>;
  const mission = str(o.mission, 120);
  if (!mission) return null;
  const location = str(o.location, 80) || undefined;
  return { ...base, mission, location };
}

export function normalizeTeacherReplacement(raw: unknown): TeacherReplacementSlot | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  if (!isValidIsoDate(o.date) || !isValidPlanningTime(o.start) || !isValidPlanningTime(o.end)) {
    return null;
  }
  if (o.start >= o.end) return null;
  const subject = str(o.subject, 120);
  if (!subject) return null;
  const classes = Array.isArray(o.classes)
    ? o.classes.map((c) => str(c, 40)).filter(Boolean).slice(0, 12)
    : [];
  return {
    id: str(o.id, 64) || uid("repl"),
    date: o.date,
    start: o.start,
    end: o.end,
    subject,
    classes,
    room: str(o.room, 40) || undefined,
    note: str(o.note, 300) || undefined,
    createdBy: str(o.createdBy, 80) || "",
    createdAt: str(o.createdAt, 40) || new Date().toISOString(),
  };
}

export function normalizePlanningException(raw: unknown): PlanningDayException | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  if (!isValidIsoDate(o.date) || !isValidPlanningTime(o.start) || !isValidPlanningTime(o.end)) {
    return null;
  }
  if (o.start >= o.end) return null;
  const kind: PlanningExceptionKind =
    o.kind === "depassement" || o.kind === "ajustement" || o.kind === "avance"
      ? o.kind
      : "ajustement";
  return {
    id: str(o.id, 64) || uid("exc"),
    date: o.date,
    start: o.start,
    end: o.end,
    kind,
    note: str(o.note, 300) || undefined,
    createdBy: str(o.createdBy, 80) || "",
    createdAt: str(o.createdAt, 40) || new Date().toISOString(),
  };
}

export function emptyTeacherPlanning(personnelId: string, updatedBy = ""): TeacherPlanningDoc {
  return {
    version: 1,
    kind: "teacher",
    personnelId,
    weekA: [],
    weekB: [],
    replacements: [],
    updatedAt: new Date().toISOString(),
    updatedBy,
  };
}

export function emptyStaffPlanning(
  personnelId: string,
  mode: "fixed" | "rotation" = "fixed",
  updatedBy = "",
): StaffPlanningDoc {
  return {
    version: 1,
    kind: "staff",
    personnelId,
    mode,
    fixedSlots: [],
    rotations:
      mode === "rotation"
        ? [{ id: uid("rot"), label: "Semaine type", startDate: null, endDate: null, slots: [] }]
        : [],
    exceptions: [],
    updatedAt: new Date().toISOString(),
    updatedBy,
  };
}

export function normalizeTeacherPlanning(
  raw: unknown,
  personnelId: string,
): TeacherPlanningDoc {
  const empty = emptyTeacherPlanning(personnelId);
  if (!raw || typeof raw !== "object") return empty;
  const o = raw as Partial<TeacherPlanningDoc>;
  const weekA = Array.isArray(o.weekA)
    ? o.weekA.map(normalizeTeacherSlot).filter((s): s is TeacherPlanningSlot => !!s)
    : [];
  const weekB = Array.isArray(o.weekB)
    ? o.weekB.map(normalizeTeacherSlot).filter((s): s is TeacherPlanningSlot => !!s)
    : [];
  const replacements = Array.isArray(o.replacements)
    ? o.replacements
        .map(normalizeTeacherReplacement)
        .filter((s): s is TeacherReplacementSlot => !!s)
        .slice(0, 200)
    : [];
  return {
    version: 1,
    kind: "teacher",
    personnelId,
    weekA,
    weekB,
    replacements,
    updatedAt: str(o.updatedAt, 40) || empty.updatedAt,
    updatedBy: str(o.updatedBy, 80),
    source: o.source === "pdf_import" || o.source === "manual" ? o.source : undefined,
    sourceFileName: str(o.sourceFileName, 180) || undefined,
  };
}

export function normalizeStaffPlanning(raw: unknown, personnelId: string): StaffPlanningDoc {
  const empty = emptyStaffPlanning(personnelId);
  if (!raw || typeof raw !== "object") return empty;
  const o = raw as Partial<StaffPlanningDoc> & Record<string, unknown>;
  const mode = o.mode === "rotation" ? "rotation" : "fixed";
  const fixedSlots = Array.isArray(o.fixedSlots)
    ? o.fixedSlots.map(normalizeStaffFixedSlot).filter((s): s is StaffFixedSlot => !!s)
    : [];
  const rotations: StaffRotationWeek[] = [];
  if (Array.isArray(o.rotations)) {
    for (const r of o.rotations) {
      if (!r || typeof r !== "object") continue;
      const row = r as Partial<StaffRotationWeek>;
      const slots = Array.isArray(row.slots)
        ? row.slots.map(normalizeStaffMissionSlot).filter((s): s is StaffMissionSlot => !!s)
        : [];
      rotations.push({
        id: str(row.id, 64) || uid("rot"),
        label: str(row.label, 80) || "Semaine",
        startDate: str(row.startDate, 20) || null,
        endDate: str(row.endDate, 20) || null,
        slots,
      });
    }
  }
  if (mode === "rotation" && rotations.length === 0) {
    rotations.push({
      id: uid("rot"),
      label: "Semaine type",
      startDate: null,
      endDate: null,
      slots: [],
    });
  }

  const exceptions = Array.isArray(o.exceptions)
    ? o.exceptions
        .map(normalizePlanningException)
        .filter((e): e is PlanningDayException => !!e)
        .slice(0, 400)
    : [];

  let annualHoursTarget: number | undefined;
  const rawTarget = (o as Record<string, unknown>).annualHoursTarget;
  if (typeof rawTarget === "number" && Number.isFinite(rawTarget)) {
    annualHoursTarget = Math.max(0, Math.min(3000, Math.round(rawTarget * 10) / 10));
  } else if (typeof rawTarget === "string" && rawTarget.trim()) {
    const n = Number(rawTarget.replace(",", "."));
    if (Number.isFinite(n)) annualHoursTarget = Math.max(0, Math.min(3000, Math.round(n * 10) / 10));
  }

  return {
    version: 1,
    kind: "staff",
    personnelId,
    mode,
    fixedSlots,
    rotations,
    annualHoursTarget,
    exceptions,
    updatedAt: str(o.updatedAt, 40) || empty.updatedAt,
    updatedBy: str(o.updatedBy, 80),
    source: o.source === "pdf_import" || o.source === "manual" ? o.source : undefined,
    sourceFileName: str(o.sourceFileName, 180) || undefined,
  };
}

export function sumWeeklyHoursFromSlots(slots: { start: string; end: string }[]): number {
  return Math.round(slots.reduce((acc, s) => acc + planningSlotHours(s.start, s.end), 0) * 10) / 10;
}

/**
 * Solde annuel estimé pour un planning staff fixe :
 * (heures semaine type × 36) − quota + delta des exceptions vs créneau type du même jour.
 */
export function estimateAnnualBalance(doc: StaffPlanningDoc): AnnualBalanceEstimate {
  const weeklyHours = sumWeeklyHoursFromSlots(doc.fixedSlots);
  const weeksFactor = PLANNING_ANNUAL_WEEKS_FACTOR;
  const projectedAnnualHours = Math.round(weeklyHours * weeksFactor * 10) / 10;
  const target = typeof doc.annualHoursTarget === "number" ? doc.annualHoursTarget : null;

  let exceptionDeltaHours = 0;
  for (const exc of doc.exceptions) {
    const day = weekdayFromIsoDate(exc.date);
    const actual = planningSlotHours(exc.start, exc.end);
    if (!day) {
      exceptionDeltaHours += actual;
      continue;
    }
    const typeSlots = doc.fixedSlots.filter((s) => s.day === day);
    const typeHours = sumWeeklyHoursFromSlots(typeSlots);
    exceptionDeltaHours += actual - typeHours;
  }
  exceptionDeltaHours = Math.round(exceptionDeltaHours * 10) / 10;

  const balanceHours =
    target == null
      ? null
      : Math.round((projectedAnnualHours + exceptionDeltaHours - target) * 10) / 10;

  return {
    weeklyHours,
    projectedAnnualHours,
    annualHoursTarget: target,
    exceptionDeltaHours,
    balanceHours,
    weeksFactor,
  };
}

/** Catégories RH OGEC → mode staff. Les professeurs utilisent le modèle teacher (Clerk). */
export function planningKindForCategory(category: string): RhPlanningKind {
  return category === "professeur" ? "teacher" : "staff";
}

/**
 * Surveillants / éducation : missions & rotations liées à la présence élèves.
 * CPE, admin, compta, maintenance : semaine type fixe + exceptions / quota.
 */
export function defaultStaffModeForCategory(category: string): "fixed" | "rotation" {
  return category === "education" ? "rotation" : "fixed";
}
