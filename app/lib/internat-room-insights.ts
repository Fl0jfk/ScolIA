import type { InternatIncident, InternatRoom, InternatStudent } from "@/app/lib/internat-types";
import { studentDisplayName } from "@/app/lib/internat-types";

export type RoomOccupancyStatus = "empty" | "partial" | "full" | "over";

export type RoomInsight = {
  room: InternatRoom;
  occupants: InternatStudent[];
  status: RoomOccupancyStatus;
  freeSlots: number;
  sanctions30d: number;
  incidents30d: number;
  remarques30d: number;
  hasWatch: boolean;
  isProblematic: boolean;
};

const WING_LABELS: Record<string, string> = {
  garcons: "Garçons",
  filles: "Filles",
  mixte: "Mixte",
  non_defini: "Non précisé",
};

export function wingLabel(wing?: string) {
  if (!wing) return WING_LABELS.non_defini;
  return WING_LABELS[wing] || wing;
}

function roomStatus(count: number, capacity: number): RoomOccupancyStatus {
  if (count > capacity) return "over";
  if (count === 0) return "empty";
  if (count >= capacity) return "full";
  return "partial";
}

function incidentCutoffIso(days = 30) {
  const d = new Date();
  d.setDate(d.getDate() - days);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export function buildRoomInsights(
  rooms: InternatRoom[],
  students: InternatStudent[],
  incidents: InternatIncident[] = [],
): RoomInsight[] {
  const active = students.filter((s) => s.actif);
  const cutoff = incidentCutoffIso(30);
  const recent = incidents.filter((i) => i.occurredAt >= cutoff);
  const incidentsByStudent = new Map<string, InternatIncident[]>();
  for (const inc of recent) {
    const list = incidentsByStudent.get(inc.studentId) || [];
    list.push(inc);
    incidentsByStudent.set(inc.studentId, list);
  }

  return rooms.map((room) => {
    const occupants = active.filter((s) => s.roomId === room.id);
    const status = roomStatus(occupants.length, room.capacity);
    const freeSlots = Math.max(0, room.capacity - occupants.length);
    let sanctions30d = 0;
    let incidents30d = 0;
    let remarques30d = 0;
    let hasWatch = false;
    for (const o of occupants) {
      if (o.underWatch) hasWatch = true;
      for (const inc of incidentsByStudent.get(o.id) || []) {
        if (inc.kind === "sanction") sanctions30d += 1;
        else if (inc.kind === "incident") incidents30d += 1;
        else if (inc.kind === "remarque") remarques30d += 1;
      }
    }
    const isProblematic =
      status === "over" || hasWatch || sanctions30d > 0 || incidents30d >= 2;
    return {
      room,
      occupants,
      status,
      freeSlots,
      sanctions30d,
      incidents30d,
      remarques30d,
      hasWatch,
      isProblematic,
    };
  });
}

type WingOccupancySummary = {
  wing: "garcons" | "filles" | "mixte" | "non_defini";
  label: string;
  roomCount: number;
  beds: number;
  occupied: number;
  fillRate: number | null;
  roomsFull: number;
  roomsEmpty: number;
};

export function buildWingOccupancy(rooms: InternatRoom[], students: InternatStudent[]): WingOccupancySummary[] {
  const active = students.filter((s) => s.actif);
  const wings: Array<WingOccupancySummary["wing"]> = ["garcons", "filles", "mixte", "non_defini"];

  return wings.map((wing) => {
    const wingRooms = rooms.filter((r) => {
      if (wing === "non_defini") return !r.wing;
      return r.wing === wing;
    });
    const roomIds = new Set(wingRooms.map((r) => r.id));
    const beds = wingRooms.reduce((s, r) => s + r.capacity, 0);
    const occupied = active.filter((s) => s.roomId && roomIds.has(s.roomId)).length;
    let roomsFull = 0;
    let roomsEmpty = 0;
    for (const room of wingRooms) {
      const c = active.filter((s) => s.roomId === room.id).length;
      if (c === 0) roomsEmpty += 1;
      else if (c >= room.capacity) roomsFull += 1;
    }
    return {
      wing,
      label: wingLabel(wing === "non_defini" ? undefined : wing),
      roomCount: wingRooms.length,
      beds,
      occupied,
      fillRate: beds > 0 ? Math.round((occupied / beds) * 100) : null,
      roomsFull,
      roomsEmpty,
    };
  }).filter((w) => w.roomCount > 0);
}

export const ROOM_STATUS_LABELS: Record<RoomOccupancyStatus, string> = {
  empty: "Vide",
  partial: "Partielle",
  full: "Pleine",
  over: "Surbooking",
};

export const ROOM_STATUS_STYLES: Record<
  RoomOccupancyStatus,
  { card: string; badge: string; dot: string }
> = {
  empty: {
    card: "border-slate-200 bg-slate-50/80",
    badge: "bg-slate-100 text-slate-600",
    dot: "bg-slate-300",
  },
  partial: {
    card: "border-sky-200 bg-sky-50/50",
    badge: "bg-sky-100 text-sky-800",
    dot: "bg-sky-400",
  },
  full: {
    card: "border-amber-200 bg-amber-50/60",
    badge: "bg-amber-100 text-amber-900",
    dot: "bg-amber-500",
  },
  over: {
    card: "border-red-300 bg-red-50/70",
    badge: "bg-red-100 text-red-800",
    dot: "bg-red-500",
  },
};

export function studentInternLink(studentId: string) {
  return `/gestion-internat?tab=internes&student=${encodeURIComponent(studentId)}`;
}

export function formatOccupantLine(s: InternatStudent) {
  return `${studentDisplayName(s)} · ${s.classe} · ${s.etablissement}`;
}
