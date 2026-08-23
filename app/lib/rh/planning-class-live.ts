import "server-only";

import { studentInAssignedClasses } from "@/app/lib/class-allocation-teachers";
import type { SchoolHolidayZone } from "@/app/lib/fr-school-holidays";
import {
  resolveDayContext,
  schoolWeekParity,
} from "@/app/lib/rh/planning-calendar";
import { readRhPlanning } from "@/app/lib/rh/planning-storage";
import {
  getTeacherPlanningEntries,
  type TeacherPlanningEntry,
} from "@/app/lib/rh/planning-teacher-index";
import {
  isPlanningWeekday,
  planningTimeToMinutes,
  type TeacherPlanningDoc,
  type TeacherPlanningSlot,
} from "@/app/lib/rh/planning-types";

export type EleveLiveCourse = {
  subject: string;
  room: string | null;
  start: string;
  end: string;
  teacherName: string;
  kind: "cours" | "remplacement";
  weekType: "A" | "B" | null;
};

export type EleveLiveCourseReason =
  | "en_cours"
  | "pas_de_classe"
  | "vacances"
  | "weekend"
  | "ferie"
  | "conge"
  | "hors_cours"
  | "pas_edt";

export type EleveLiveCourseResult = {
  activity: EleveLiveCourse | null;
  reason: EleveLiveCourseReason;
  label?: string;
  /** Plusieurs créneaux EDT coïncident (conflit de saisie). */
  conflictCount?: number;
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
  const map: Record<string, 1 | 2 | 3 | 4 | 5> = {
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

function slotCoversTime(slot: { start: string; end: string }, time: string): boolean {
  return slot.start <= time && time < slot.end;
}

function slotMatchesClass(slot: TeacherPlanningSlot, classe: string): boolean {
  return studentInAssignedClasses(classe, slot.classes || []);
}

function replacementAsSlot(
  r: TeacherPlanningDoc["replacements"][number],
): TeacherPlanningSlot {
  return {
    id: r.id,
    day: 1,
    start: r.start,
    end: r.end,
    subject: r.subject,
    classes: r.classes,
    room: r.room,
  };
}

/** Classes distinctes présentes dans l’EDT prof (semaines types + remplacements). */
export function classesFromTeacherPlanning(doc: TeacherPlanningDoc): string[] {
  const set = new Set<string>();
  for (const slot of [...doc.weekA, ...doc.weekB]) {
    for (const c of slot.classes || []) {
      const t = c.trim();
      if (t) set.add(t);
    }
  }
  for (const r of doc.replacements || []) {
    for (const c of r.classes || []) {
      const t = c.trim();
      if (t) set.add(t);
    }
  }
  return [...set].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

function teacherPlanningHasContent(doc: TeacherPlanningDoc): boolean {
  return (
    doc.weekA.length > 0 ||
    doc.weekB.length > 0 ||
    (doc.replacements?.length ?? 0) > 0
  );
}

function findLiveInTeacherPlanning(input: {
  classe: string;
  entry: TeacherPlanningEntry;
  time: string;
  date: string;
  day: 1 | 2 | 3 | 4 | 5;
  weekType: "A" | "B";
}): EleveLiveCourse | null {
  const { classe, entry, time, date, day, weekType } = input;
  const doc = entry.planning;

  for (const r of doc.replacements || []) {
    if (r.date !== date) continue;
    const slot = replacementAsSlot(r);
    if (!slotCoversTime(slot, time) || !slotMatchesClass(slot, classe)) continue;
    return {
      subject: r.subject,
      room: r.room || null,
      start: r.start,
      end: r.end,
      teacherName: entry.displayName,
      kind: "remplacement",
      weekType: null,
    };
  }

  const typeSlots = weekType === "A" ? doc.weekA : doc.weekB;
  for (const slot of typeSlots) {
    if (slot.day !== day) continue;
    if (!slotCoversTime(slot, time) || !slotMatchesClass(slot, classe)) continue;
    return {
      subject: slot.subject,
      room: slot.room || null,
      start: slot.start,
      end: slot.end,
      teacherName: entry.displayName,
      kind: "cours",
      weekType,
    };
  }

  return null;
}

/** Cours en direct pour une classe d’élève (agrégation EDT profs). */
export async function resolveEleveLiveCourse(input: {
  classe: string | null | undefined;
  zone?: SchoolHolidayZone | null;
  now?: Date;
  teacherEntries?: TeacherPlanningEntry[];
}): Promise<EleveLiveCourseResult> {
  const classe = input.classe?.trim();
  if (!classe) {
    return { activity: null, reason: "pas_de_classe" };
  }

  const now = input.now ?? new Date();
  const { time, date, day } = parisNowParts(now);
  const ctx = resolveDayContext({
    isoDate: date,
    audience: "teacher",
    zone: input.zone ?? null,
    leaves: [],
  });

  if (ctx.kind === "school_holiday") {
    return { activity: null, reason: "vacances", label: ctx.label };
  }
  if (ctx.kind === "weekend") {
    return { activity: null, reason: "weekend", label: ctx.label };
  }
  if (ctx.kind === "ferie") {
    return { activity: null, reason: "ferie", label: ctx.label };
  }
  if (ctx.kind === "leave") {
    return { activity: null, reason: "conge", label: ctx.label };
  }
  if (!day || !isPlanningWeekday(day)) {
    return { activity: null, reason: "hors_cours" };
  }

  const entries = input.teacherEntries ?? (await getTeacherPlanningEntries());
  if (!entries.length) {
    return { activity: null, reason: "pas_edt" };
  }

  const weekType = schoolWeekParity(now);
  const hits: EleveLiveCourse[] = [];

  for (const entry of entries) {
    const hit = findLiveInTeacherPlanning({
      classe,
      entry,
      time,
      date,
      day,
      weekType,
    });
    if (hit) hits.push(hit);
  }

  if (!hits.length) {
    const anyClassInEdt = entries.some((e) =>
      classesFromTeacherPlanning(e.planning).some((c) =>
        studentInAssignedClasses(classe, [c]),
      ),
    );
    return {
      activity: null,
      reason: anyClassInEdt ? "hors_cours" : "pas_edt",
    };
  }

  hits.sort((a, b) => {
    const pa = planningTimeToMinutes(a.start);
    const pb = planningTimeToMinutes(b.start);
    if (pa !== pb) return pa - pb;
    return a.subject.localeCompare(b.subject, "fr");
  });

  return {
    activity: hits[0]!,
    reason: "en_cours",
    conflictCount: hits.length > 1 ? hits.length : undefined,
  };
}

/** Classes enseignées par un prof d’après son EDT (P2). */
export async function listClassesFromTeacherEdt(externalUserId: string): Promise<string[]> {
  const doc = await readRhPlanning("teacher", externalUserId);
  if (doc.kind !== "teacher" || !teacherPlanningHasContent(doc)) return [];
  return classesFromTeacherPlanning(doc);
}
