import "server-only";

import {
  addDays,
  schoolWeekParity,
  startOfWeekMonday,
  toIsoDateLocal,
} from "@/app/lib/rh/planning-calendar";
import {
  getTeacherPlanningEntries,
  type TeacherPlanningEntry,
} from "@/app/lib/rh/planning-teacher-index";
import {
  isPlanningWeekday,
  planningTimeToMinutes,
  type PlanningWeekday,
  type TeacherPlanningSlot,
} from "@/app/lib/rh/planning-types";
import type { ReservationRoomRow } from "@/app/lib/reservation-rooms-db";

export type EdtRoomOccupancyCell = {
  date: string;
  /** Heure calendrier réservation (8 = cellule 8h30–9h30). */
  hour: number;
  start: string;
  end: string;
  subject: string;
  classes: string[];
  teacherName: string;
  weekType: "A" | "B" | "replacement";
  room: string;
  source: "edt" | "replacement";
};

export type EdtRoomOccupancyResult = {
  from: string;
  to: string;
  roomId: string;
  roomName: string;
  weekABByDate: Record<string, "A" | "B">;
  cells: EdtRoomOccupancyCell[];
};

function normalizeRoomKey(value: string | null | undefined): string {
  return (value || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .trim()
    .toLowerCase()
    .replace(/\s+/g, " ");
}

function roomAliases(room: Pick<ReservationRoomRow, "id" | "name">): Set<string> {
  const keys = new Set<string>();
  const id = normalizeRoomKey(room.id);
  const name = normalizeRoomKey(room.name);
  if (id) keys.add(id);
  if (name) keys.add(name);
  return keys;
}

function slotMatchesRoom(slotRoom: string | undefined, aliases: Set<string>): boolean {
  const key = normalizeRoomKey(slotRoom);
  if (!key || aliases.size === 0) return false;
  if (aliases.has(key)) return true;
  for (const a of aliases) {
    if (a && (key === a || key.includes(a) || a.includes(key))) return true;
  }
  return false;
}

/** Cellules Hh30 du calendrier réservation qui chevauchent [start, end]. */
export function reservationHoursCoveredBySlot(start: string, end: string): number[] {
  const s = planningTimeToMinutes(start);
  const e = planningTimeToMinutes(end);
  if (!(e > s)) return [];
  const hours: number[] = [];
  for (let h = 8; h <= 17; h += 1) {
    const cellStart = h * 60 + 30;
    const cellEnd = (h + 1) * 60 + 30;
    if (s < cellEnd && e > cellStart) hours.push(h);
  }
  return hours;
}

function weekdayFromIsoDate(iso: string): PlanningWeekday | null {
  const [y, m, d] = iso.split("-").map(Number);
  const date = new Date(y!, (m || 1) - 1, d || 1);
  const js = date.getDay();
  if (js === 0 || js === 6) return null;
  const day = js as PlanningWeekday;
  return isPlanningWeekday(day) ? day : null;
}

function eachDateInclusive(from: string, to: string): string[] {
  const out: string[] = [];
  let cur = parseIsoLocal(from);
  const end = parseIsoLocal(to);
  if (Number.isNaN(cur.getTime()) || Number.isNaN(end.getTime())) return out;
  while (cur <= end) {
    out.push(toIsoDateLocal(cur));
    cur = addDays(cur, 1);
  }
  return out;
}

function parseIsoLocal(iso: string): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y!, (m || 1) - 1, d || 1);
}

function pushSlotCells(opts: {
  date: string;
  slot: Pick<TeacherPlanningSlot, "start" | "end" | "subject" | "classes" | "room">;
  teacherName: string;
  weekType: EdtRoomOccupancyCell["weekType"];
  source: EdtRoomOccupancyCell["source"];
  out: EdtRoomOccupancyCell[];
}) {
  const room = (opts.slot.room || "").trim();
  for (const hour of reservationHoursCoveredBySlot(opts.slot.start, opts.slot.end)) {
    opts.out.push({
      date: opts.date,
      hour,
      start: opts.slot.start,
      end: opts.slot.end,
      subject: opts.slot.subject || "Cours",
      classes: opts.slot.classes || [],
      teacherName: opts.teacherName,
      weekType: opts.weekType,
      room,
      source: opts.source,
    });
  }
}

export function buildEdtRoomOccupancy(opts: {
  entries: TeacherPlanningEntry[];
  room: Pick<ReservationRoomRow, "id" | "name">;
  from: string;
  to: string;
}): EdtRoomOccupancyResult {
  const aliases = roomAliases(opts.room);
  const weekABByDate: Record<string, "A" | "B"> = {};
  const cells: EdtRoomOccupancyCell[] = [];

  for (const date of eachDateInclusive(opts.from, opts.to)) {
    const parity = schoolWeekParity(parseIsoLocal(date));
    weekABByDate[date] = parity;
    const day = weekdayFromIsoDate(date);
    if (!day) continue;

    for (const entry of opts.entries) {
      const typeSlots = parity === "A" ? entry.planning.weekA : entry.planning.weekB;
      for (const slot of typeSlots) {
        if (slot.day !== day) continue;
        if (!slotMatchesRoom(slot.room, aliases)) continue;
        pushSlotCells({
          date,
          slot,
          teacherName: entry.displayName,
          weekType: parity,
          source: "edt",
          out: cells,
        });
      }
      for (const rep of entry.planning.replacements || []) {
        if (rep.date !== date) continue;
        if (!slotMatchesRoom(rep.room, aliases)) continue;
        pushSlotCells({
          date,
          slot: rep,
          teacherName: entry.displayName,
          weekType: "replacement",
          source: "replacement",
          out: cells,
        });
      }
    }
  }

  return {
    from: opts.from,
    to: opts.to,
    roomId: opts.room.id,
    roomName: opts.room.name,
    weekABByDate,
    cells,
  };
}

export async function loadEdtRoomOccupancy(opts: {
  room: Pick<ReservationRoomRow, "id" | "name">;
  from: string;
  to: string;
}): Promise<EdtRoomOccupancyResult> {
  const entries = await getTeacherPlanningEntries();
  return buildEdtRoomOccupancy({ ...opts, entries });
}

/** Plage par défaut autour d’une date (semaine affichée). */
export function occupancyRangeForWeek(anchor: Date): { from: string; to: string } {
  const monday = startOfWeekMonday(anchor);
  const friday = addDays(monday, 4);
  return { from: toIsoDateLocal(monday), to: toIsoDateLocal(friday) };
}

export function edtOccupancyConflictsWithHours(opts: {
  occupancy: EdtRoomOccupancyResult;
  date: string;
  hours: number[];
}): EdtRoomOccupancyCell[] {
  const wanted = new Set(opts.hours);
  return opts.occupancy.cells.filter(
    (c) => c.date === opts.date && wanted.has(c.hour),
  );
}
