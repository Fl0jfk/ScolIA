import "server-only";

import {
  PLANNING_WEEKDAY_LABELS,
  planningTimeToMinutes,
  type PlanningWeekday,
  type TeacherPlanningSlot,
} from "@/app/lib/rh/planning-types";
import type { TeacherPlanningEntry } from "@/app/lib/rh/planning-teacher-index";

export type PlanningConflictKind = "room" | "class";

export type PlanningConflict = {
  kind: PlanningConflictKind;
  weekType: "A" | "B";
  day: PlanningWeekday;
  start: string;
  end: string;
  message: string;
  slotIds: string[];
  teacherIds: string[];
  teacherNames: string[];
  room: string | null;
  classes: string[];
  subjects: string[];
};

type ExpandedSlot = {
  slot: TeacherPlanningSlot;
  personnelId: string;
  displayName: string;
  weekType: "A" | "B";
};

function normalizeRoom(room: string | null | undefined): string | null {
  const value = room?.trim();
  return value ? value.toUpperCase() : null;
}

function normalizeClassName(classe: string): string {
  return classe.trim().toLowerCase();
}

function slotsOverlap(a: TeacherPlanningSlot, b: TeacherPlanningSlot): boolean {
  if (a.day !== b.day) return false;
  return (
    planningTimeToMinutes(a.end) > planningTimeToMinutes(b.start) &&
    planningTimeToMinutes(b.end) > planningTimeToMinutes(a.start)
  );
}

function overlapEnd(a: TeacherPlanningSlot, b: TeacherPlanningSlot): string {
  return planningTimeToMinutes(a.end) >= planningTimeToMinutes(b.end) ? a.end : b.end;
}

function expandWeekSlots(
  entries: TeacherPlanningEntry[],
  weekType: "A" | "B",
): ExpandedSlot[] {
  const out: ExpandedSlot[] = [];
  for (const entry of entries) {
    const slots = weekType === "A" ? entry.planning.weekA : entry.planning.weekB;
    for (const slot of slots) {
      out.push({
        slot,
        personnelId: entry.personnelId,
        displayName: entry.displayName,
        weekType,
      });
    }
  }
  return out;
}

function sharedClasses(a: TeacherPlanningSlot, b: TeacherPlanningSlot): string[] {
  const setB = new Set((b.classes || []).map(normalizeClassName));
  const shared: string[] = [];
  for (const c of a.classes || []) {
    const key = normalizeClassName(c);
    if (setB.has(key)) shared.push(c.trim());
  }
  return [...new Set(shared)];
}

function conflictKey(parts: string[]): string {
  return parts.join("|");
}

/** Détecte les conflits inter-profs (salle ou classe) pour une semaine type A ou B. */
export function findCrossTeacherPlanningConflicts(
  entries: TeacherPlanningEntry[],
  weekType: "A" | "B",
): PlanningConflict[] {
  const expanded = expandWeekSlots(entries, weekType);
  const conflicts: PlanningConflict[] = [];
  const seen = new Set<string>();

  for (let i = 0; i < expanded.length; i += 1) {
    for (let j = i + 1; j < expanded.length; j += 1) {
      const a = expanded[i]!;
      const b = expanded[j]!;
      if (a.personnelId === b.personnelId) continue;
      if (!slotsOverlap(a.slot, b.slot)) continue;

      const dayLabel = PLANNING_WEEKDAY_LABELS[a.slot.day];
      const start = a.slot.start;
      const end = overlapEnd(a.slot, b.slot);

      const roomA = normalizeRoom(a.slot.room);
      const roomB = normalizeRoom(b.slot.room);
      if (roomA && roomB && roomA === roomB) {
        const key = conflictKey([
          "room",
          weekType,
          String(a.slot.day),
          start,
          roomA,
          ...[a.personnelId, b.personnelId].sort(),
        ]);
        if (!seen.has(key)) {
          seen.add(key);
          conflicts.push({
            kind: "room",
            weekType,
            day: a.slot.day,
            start,
            end,
            message: `${dayLabel} ${start}–${end} · salle ${roomA} · ${a.displayName} (${a.slot.subject}) / ${b.displayName} (${b.slot.subject})`,
            slotIds: [a.slot.id, b.slot.id],
            teacherIds: [a.personnelId, b.personnelId],
            teacherNames: [a.displayName, b.displayName],
            room: roomA,
            classes: [],
            subjects: [a.slot.subject, b.slot.subject],
          });
        }
      }

      const classes = sharedClasses(a.slot, b.slot);
      if (classes.length > 0) {
        const key = conflictKey([
          "class",
          weekType,
          String(a.slot.day),
          start,
          ...classes.map(normalizeClassName).sort(),
          ...[a.personnelId, b.personnelId].sort(),
        ]);
        if (!seen.has(key)) {
          seen.add(key);
          conflicts.push({
            kind: "class",
            weekType,
            day: a.slot.day,
            start,
            end,
            message: `${dayLabel} ${start}–${end} · ${classes.join(", ")} · ${a.displayName} (${a.slot.subject}) / ${b.displayName} (${b.slot.subject})`,
            slotIds: [a.slot.id, b.slot.id],
            teacherIds: [a.personnelId, b.personnelId],
            teacherNames: [a.displayName, b.displayName],
            room: roomA || roomB,
            classes,
            subjects: [a.slot.subject, b.slot.subject],
          });
        }
      }
    }
  }

  return conflicts.sort((x, y) => {
    if (x.day !== y.day) return x.day - y.day;
    const tx = planningTimeToMinutes(x.start);
    const ty = planningTimeToMinutes(y.start);
    if (tx !== ty) return tx - ty;
    return x.message.localeCompare(y.message, "fr");
  });
}

/** Conflits semaines A et B combinés. */
export function findAllCrossTeacherPlanningConflicts(
  entries: TeacherPlanningEntry[],
): PlanningConflict[] {
  return [
    ...findCrossTeacherPlanningConflicts(entries, "A"),
    ...findCrossTeacherPlanningConflicts(entries, "B"),
  ];
}

/** Conflits impliquant un prof donné (pour retour API sauvegarde). */
export function conflictsForTeacher(
  conflicts: PlanningConflict[],
  personnelId: string,
): PlanningConflict[] {
  return conflicts.filter((c) => c.teacherIds.includes(personnelId));
}
