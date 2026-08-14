/** Helpers calendrier planning RH — fusion, semaine, vacances, activité en cours. */

import { isJourFerieFranceMetropole } from "@/app/lib/fr-public-holidays";
import {
  isWeekendIsoDate,
  schoolHolidayOnDate,
  type SchoolHolidayZone,
} from "@/app/lib/fr-school-holidays";
import {
  isPlanningWeekday,
  planningSlotHours,
  planningTimeToMinutes,
  type PlanningWeekday,
  type RhPlanningDoc,
  type StaffPlanningDoc,
  type TeacherPlanningDoc,
  type TeacherReplacementSlot,
} from "@/app/lib/rh/planning-types";

export type CalendarBlock = {
  id: string;
  day: PlanningWeekday;
  start: string;
  end: string;
  title: string;
  subtitle?: string;
  kind: "type" | "replacement" | "exception" | "mission" | "leave";
};

export type DayContextKind = "weekend" | "ferie" | "school_holiday" | "leave" | "work";

export type DayContext = {
  date: string;
  kind: DayContextKind;
  label: string;
  /** Pour les profs : masquer / griser l’emploi du temps type. */
  suppressTypeSlots: boolean;
};

export type LeaveSpan = {
  startDate: string;
  endDate: string;
  type: string;
  label: string;
};

/** Semaine ISO (1–53) — paire = A, impaire = B (convention établissement v1). */
export function schoolWeekParity(d = new Date()): "A" | "B" {
  const week = isoWeekNumber(d);
  return week % 2 === 0 ? "A" : "B";
}

function isoWeekNumber(d: Date): number {
  const date = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
  const dayNum = date.getUTCDay() || 7;
  date.setUTCDate(date.getUTCDate() + 4 - dayNum);
  const yearStart = new Date(Date.UTC(date.getUTCFullYear(), 0, 1));
  return Math.ceil(((date.getTime() - yearStart.getTime()) / 86400000 + 1) / 7);
}

/** Lundi 00:00 local de la semaine contenant `d`. */
export function startOfWeekMonday(d: Date): Date {
  const x = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  const day = x.getDay();
  const diff = day === 0 ? -6 : 1 - day;
  x.setDate(x.getDate() + diff);
  x.setHours(0, 0, 0, 0);
  return x;
}

export function addDays(d: Date, n: number): Date {
  const x = new Date(d);
  x.setDate(x.getDate() + n);
  return x;
}

export function toIsoDateLocal(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function weekdayFromDate(d: Date): PlanningWeekday | null {
  const js = d.getDay();
  if (js === 0 || js === 6) return null;
  return js as PlanningWeekday;
}

function resolveDayContext(input: {
  isoDate: string;
  audience: "teacher" | "staff";
  zone?: SchoolHolidayZone | null;
  leaves?: LeaveSpan[];
}): DayContext {
  const { isoDate, audience, zone = null, leaves = [] } = input;

  if (isWeekendIsoDate(isoDate)) {
    return {
      date: isoDate,
      kind: "weekend",
      label: "Week-end",
      suppressTypeSlots: true,
    };
  }

  const d = new Date(`${isoDate}T12:00:00`);
  if (isJourFerieFranceMetropole(d)) {
    return {
      date: isoDate,
      kind: "ferie",
      label: "Jour férié",
      suppressTypeSlots: true,
    };
  }

  const leave = leaves.find((l) => isoDate >= l.startDate && isoDate <= l.endDate);
  if (leave) {
    return {
      date: isoDate,
      kind: "leave",
      label: leave.label,
      suppressTypeSlots: true,
    };
  }

  const hol = schoolHolidayOnDate(isoDate, zone);
  if (hol) {
    return {
      date: isoDate,
      kind: "school_holiday",
      label: hol.label,
      // Profs : pas de cours type. Personnel : souvent au travail sauf CP/RTT.
      suppressTypeSlots: audience === "teacher",
    };
  }

  return {
    date: isoDate,
    kind: "work",
    label: "",
    suppressTypeSlots: false,
  };
}

/** Fusionne les créneaux adjacents de même titre/sous-titre le même jour. */
function mergeAdjacentBlocks(blocks: CalendarBlock[]): CalendarBlock[] {
  const sorted = [...blocks].sort((a, b) => {
    if (a.day !== b.day) return a.day - b.day;
    return a.start.localeCompare(b.start);
  });
  const out: CalendarBlock[] = [];
  for (const b of sorted) {
    const prev = out[out.length - 1];
    if (
      prev &&
      prev.day === b.day &&
      prev.end === b.start &&
      prev.title === b.title &&
      (prev.subtitle || "") === (b.subtitle || "") &&
      prev.kind === b.kind
    ) {
      prev.end = b.end;
      continue;
    }
    out.push({ ...b });
  }
  return out;
}

function teacherTypeBlocks(doc: TeacherPlanningDoc, week: "A" | "B"): CalendarBlock[] {
  const slots = week === "A" ? doc.weekA : doc.weekB;
  return slots.map((s) => ({
    id: s.id,
    day: s.day,
    start: s.start,
    end: s.end,
    title: s.subject,
    subtitle: [s.classes.join(", "), s.room].filter(Boolean).join(" · ") || undefined,
    kind: "type" as const,
  }));
}

function staffFixedBlocks(doc: StaffPlanningDoc): CalendarBlock[] {
  return doc.fixedSlots.map((s) => ({
    id: s.id,
    day: s.day,
    start: s.start,
    end: s.end,
    title: s.label,
    kind: "type" as const,
  }));
}

function staffMissionBlocks(doc: StaffPlanningDoc, rotationId?: string): CalendarBlock[] {
  const rot = doc.rotations.find((r) => r.id === rotationId) || doc.rotations[0] || null;
  if (!rot) return [];
  return rot.slots.map((s) => ({
    id: s.id,
    day: s.day,
    start: s.start,
    end: s.end,
    title: s.mission,
    subtitle: s.location,
    kind: "mission" as const,
  }));
}

export function blocksForPlanningWeek(input: {
  planning: RhPlanningDoc;
  weekAB?: "A" | "B";
  rotationId?: string;
  weekStart: Date;
  zone?: SchoolHolidayZone | null;
  leaves?: LeaveSpan[];
}): CalendarBlock[] {
  const {
    planning,
    weekAB = "A",
    rotationId,
    weekStart,
    zone = null,
    leaves = [],
  } = input;
  const audience = planning.kind === "teacher" ? "teacher" : "staff";
  let base: CalendarBlock[] = [];

  if (planning.kind === "teacher") {
    base = teacherTypeBlocks(planning, weekAB);
    const weekDates = [0, 1, 2, 3, 4].map((i) => toIsoDateLocal(addDays(weekStart, i)));
    const repls = (planning.replacements || []).filter((r) => weekDates.includes(r.date));
    for (const r of repls) {
      const d = new Date(`${r.date}T12:00:00`);
      const day = weekdayFromDate(d);
      if (!day) continue;
      base.push({
        id: r.id,
        day,
        start: r.start,
        end: r.end,
        title: r.subject,
        subtitle: ["Remplacement", r.classes.join(", "), r.room].filter(Boolean).join(" · "),
        kind: "replacement",
      });
    }
  } else if (planning.mode === "fixed") {
    base = staffFixedBlocks(planning);
    const weekDates = [0, 1, 2, 3, 4].map((i) => toIsoDateLocal(addDays(weekStart, i)));
    for (const e of planning.exceptions || []) {
      if (!weekDates.includes(e.date)) continue;
      const d = new Date(`${e.date}T12:00:00`);
      const day = weekdayFromDate(d);
      if (!day) continue;
      base = base.filter((b) => b.day !== day);
      base.push({
        id: e.id,
        day,
        start: e.start,
        end: e.end,
        title: e.kind === "avance" ? "Horaires (avance)" : "Horaires ajustés",
        subtitle: e.note,
        kind: "exception",
      });
    }
  } else {
    base = staffMissionBlocks(planning, rotationId);
  }

  base = base.filter((b) => {
    const date = toIsoDateLocal(addDays(weekStart, b.day - 1));
    const ctx = resolveDayContext({ isoDate: date, audience, zone, leaves });
    if (!ctx.suppressTypeSlots) return true;
    return b.kind === "replacement";
  });

  return mergeAdjacentBlocks(base);
}

export function weekDayContexts(input: {
  weekStart: Date;
  audience: "teacher" | "staff";
  zone?: SchoolHolidayZone | null;
  leaves?: LeaveSpan[];
  includeWeekend?: boolean;
}): DayContext[] {
  const days = input.includeWeekend ? 7 : 5;
  const out: DayContext[] = [];
  for (let i = 0; i < days; i++) {
    const date = toIsoDateLocal(addDays(input.weekStart, i));
    out.push(
      resolveDayContext({
        isoDate: date,
        audience: input.audience,
        zone: input.zone,
        leaves: input.leaves,
      }),
    );
  }
  return out;
}

type CurrentActivity = {
  title: string;
  detail: string;
  start: string;
  end: string;
  hours: number;
};

function parisNowParts(now = new Date()) {
  const fmt = new Intl.DateTimeFormat("fr-FR", {
    timeZone: "Europe/Paris",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
    weekday: "short",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
  const parts = Object.fromEntries(fmt.formatToParts(now).map((p) => [p.type, p.value]));
  const time = `${parts.hour}:${parts.minute}`;
  const date = `${parts.year}-${parts.month}-${parts.day}`;
  const map: Record<string, PlanningWeekday> = {
    lun: 1,
    mar: 2,
    mer: 3,
    jeu: 4,
    ven: 5,
  };
  const w = (parts.weekday || "").toLowerCase().replace(".", "").slice(0, 3);
  const day = map[w] ?? null;
  return { time, date, day };
}

export function findCurrentActivity(
  planning: RhPlanningDoc,
  now = new Date(),
  weekAB?: "A" | "B",
  leaves?: LeaveSpan[],
  zone?: SchoolHolidayZone | null,
): CurrentActivity | null {
  const { time, date, day } = parisNowParts(now);
  const audience = planning.kind === "teacher" ? "teacher" : "staff";
  const ctx = resolveDayContext({ isoDate: date, audience, leaves, zone });
  if (ctx.kind === "leave") {
    return {
      title: ctx.label,
      detail: "Congé déclaré",
      start: "—",
      end: "—",
      hours: 0,
    };
  }
  if (ctx.suppressTypeSlots) return null;
  if (!day || !isPlanningWeekday(day)) return null;

  const weekStart = startOfWeekMonday(now);
  const blocks = blocksForPlanningWeek({
    planning,
    weekAB: weekAB || schoolWeekParity(now),
    weekStart,
    leaves,
    zone,
  });

  const hit = blocks.find((b) => b.day === day && b.start <= time && time < b.end);
  if (!hit) {
    if (planning.kind === "teacher") {
      const r = (planning.replacements || []).find(
        (x: TeacherReplacementSlot) => x.date === date && x.start <= time && time < x.end,
      );
      if (r) {
        return {
          title: r.subject,
          detail: ["Remplacement", r.classes.join(", "), r.room].filter(Boolean).join(" · "),
          start: r.start,
          end: r.end,
          hours: planningSlotHours(r.start, r.end),
        };
      }
    }
    return null;
  }

  return {
    title: hit.title,
    detail: hit.subtitle || `${hit.start}–${hit.end}`,
    start: hit.start,
    end: hit.end,
    hours: planningSlotHours(hit.start, hit.end),
  };
}

function minutesSinceMidnight(t: string): number {
  return planningTimeToMinutes(t);
}
