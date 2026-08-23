import "server-only";

import { studentInAssignedClasses } from "@/app/lib/class-allocation-teachers";
import { addDays, toIsoDateLocal } from "@/app/lib/rh/planning-calendar";
import type { TeacherPlanningEntry } from "@/app/lib/rh/planning-teacher-index";
import { classesFromTeacherPlanning } from "@/app/lib/rh/planning-class-live";
import {
  planningTimeToMinutes,
  type PlanningWeekday,
  type TeacherPlanningSlot,
} from "@/app/lib/rh/planning-types";

export type ClassScheduleSlot = {
  id: string;
  day: PlanningWeekday;
  start: string;
  end: string;
  subject: string;
  room: string | null;
  teacherName: string;
  teacherId: string;
  kind: "cours" | "remplacement";
  weekType: "A" | "B" | null;
  replacementDate?: string;
};

function weekdayFromIsoDate(date: string): PlanningWeekday | null {
  const d = new Date(`${date}T12:00:00`);
  if (Number.isNaN(d.getTime())) return null;
  const js = d.getDay();
  if (js === 0 || js === 6) return null;
  return js as PlanningWeekday;
}

function slotMatchesClass(slot: TeacherPlanningSlot, classe: string): boolean {
  return studentInAssignedClasses(classe, slot.classes || []);
}

/** Liste triée des classes présentes dans l’index EDT profs. */
export function listClassesFromTeacherIndex(entries: TeacherPlanningEntry[]): string[] {
  const set = new Set<string>();
  for (const entry of entries) {
    for (const c of classesFromTeacherPlanning(entry.planning)) {
      set.add(c);
    }
  }
  return [...set].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

/** Emploi du temps hebdo agrégé pour une classe (semaine type + remplacements de la semaine calendaire). */
export function buildClassWeekSchedule(input: {
  classe: string;
  entries: TeacherPlanningEntry[];
  weekType: "A" | "B";
  weekStart: Date;
}): ClassScheduleSlot[] {
  const { classe, entries, weekType, weekStart } = input;
  const weekDates = [0, 1, 2, 3, 4].map((i) => toIsoDateLocal(addDays(weekStart, i)));
  const out: ClassScheduleSlot[] = [];

  for (const entry of entries) {
    const doc = entry.planning;

    for (const r of doc.replacements || []) {
      if (!weekDates.includes(r.date)) continue;
      const day = weekdayFromIsoDate(r.date);
      if (!day) continue;
      const slot: TeacherPlanningSlot = {
        id: r.id,
        day,
        start: r.start,
        end: r.end,
        subject: r.subject,
        classes: r.classes,
        room: r.room,
      };
      if (!slotMatchesClass(slot, classe)) continue;
      out.push({
        id: `repl_${entry.personnelId}_${r.id}`,
        day,
        start: r.start,
        end: r.end,
        subject: r.subject,
        room: r.room || null,
        teacherName: entry.displayName,
        teacherId: entry.personnelId,
        kind: "remplacement",
        weekType: null,
        replacementDate: r.date,
      });
    }

    const typeSlots = weekType === "A" ? doc.weekA : doc.weekB;
    for (const slot of typeSlots) {
      if (!slotMatchesClass(slot, classe)) continue;
      out.push({
        id: `${entry.personnelId}_${slot.id}`,
        day: slot.day,
        start: slot.start,
        end: slot.end,
        subject: slot.subject,
        room: slot.room || null,
        teacherName: entry.displayName,
        teacherId: entry.personnelId,
        kind: "cours",
        weekType,
      });
    }
  }

  return out.sort((a, b) => {
    if (a.day !== b.day) return a.day - b.day;
    const ta = planningTimeToMinutes(a.start);
    const tb = planningTimeToMinutes(b.start);
    if (ta !== tb) return ta - tb;
    return a.subject.localeCompare(b.subject, "fr");
  });
}