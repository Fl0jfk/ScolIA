import type {
  InternatIncident,
  InternatRollCall,
  InternatRollMark,
  InternatRoom,
  InternatStudent,
} from "@/app/lib/internat-types";
import { studentDisplayName } from "@/app/lib/internat-types";
import { buildRoomInsights, buildWingOccupancy } from "@/app/lib/internat-room-insights";

export type InternatDashboardStats = {
  activeStudents: number;
  students: {
    boys: number;
    girls: number;
    college: number;
    lycee: number;
    withoutRoom: number;
  };
  roomCount: number;
  occupancy: {
    totalBeds: number;
    occupiedBeds: number;
    fillRate: number | null;
    roomsFull: number;
    roomsEmpty: number;
    roomsPartial: number;
  };
  wingOccupancy: Array<{
    wing: string;
    label: string;
    roomCount: number;
    beds: number;
    occupied: number;
    fillRate: number | null;
    roomsFull: number;
    roomsEmpty: number;
  }>;
  roomsOverCapacity: Array<{ roomId: string; label: string; count: number; capacity: number }>;
  roomsWithFreeSlots: number;
  problematicRoomCount: number;
  tonightRollCall: {
    date: string;
    status: "non_demarre" | "en_cours" | "validee";
    presentCount: number;
    absentCount: number;
    excusedCount: number;
    activityCount: number;
    boysComplete: boolean;
    girlsComplete: boolean;
  };
  presenceRate7d: number | null;
  lastValidatedRollCall?: { date: string; validatedBy?: string; validatedAt?: string };
  weeklySummary?: string;
  incidents30d: {
    incident: number;
    remarque: number;
    sanction: number;
    valorisation: number;
    total: number;
  };
  studentsUnderWatch: Array<{ id: string; name: string; classe: string; note?: string }>;
};

function pad2(n: number) {
  return String(n).padStart(2, "0");
}

export function todayDateParis() {
  const d = new Date();
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

function countMarks(marks: Record<string, InternatRollMark>) {
  let present = 0;
  let absent = 0;
  let excused = 0;
  let activity = 0;
  for (const v of Object.values(marks)) {
    if (v === "present") present += 1;
    else if (v === "absent") absent += 1;
    else if (v === "excuse") excused += 1;
    else if (v === "activite") activity += 1;
  }
  return { present, absent, excused, activity };
}

function sectionNeeded(students: InternatStudent[], sexe: "M" | "F") {
  return students.some((s) => s.actif && s.sexe === sexe);
}

export function sectionIsComplete(
  section: InternatRollCall["boys"],
  students: InternatStudent[],
  sexe: "M" | "F",
) {
  if (!sectionNeeded(students, sexe)) return true;
  if (!section.completed) return false;
  const active = students.filter((s) => s.actif && s.sexe === sexe);
  return active.every((s) => section.marks[s.id]);
}

export function rollCallCanValidate(rollCall: InternatRollCall, students: InternatStudent[]) {
  if (rollCall.status === "validee") return false;
  return (
    sectionIsComplete(rollCall.boys, students, "M") &&
    sectionIsComplete(rollCall.girls, students, "F")
  );
}

function computePresenceRate7d(
  rollCalls: InternatRollCall[],
  students: InternatStudent[],
  refDate = todayDateParis(),
): number | null {
  const activeIds = new Set(students.filter((s) => s.actif).map((s) => s.id));
  if (activeIds.size === 0) return null;

  const ref = new Date(refDate);
  const dates: string[] = [];
  for (let i = 0; i < 7; i += 1) {
    const d = new Date(ref);
    d.setDate(d.getDate() - i);
    dates.push(`${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`);
  }

  const validated = rollCalls.filter((r) => r.status === "validee" && dates.includes(r.date));
  if (validated.length === 0) return null;

  let expected = 0;
  let present = 0;
  for (const rc of validated) {
    for (const id of activeIds) {
      const student = students.find((s) => s.id === id);
      if (!student) continue;
      const mark =
        student.sexe === "M" ? rc.boys.marks[id] : rc.girls.marks[id];
      if (!mark) continue;
      expected += 1;
      if (mark === "present" || mark === "activite") present += 1;
    }
  }
  if (expected === 0) return null;
  return Math.round((present / expected) * 100);
}

function buildWeeklySummary(
  rollCalls: InternatRollCall[],
  students: InternatStudent[],
  rooms: InternatRoom[],
  overCapacity: InternatDashboardStats["roomsOverCapacity"],
): string {
  const validated = rollCalls.filter((r) => r.status === "validee").slice(-7);
  let absents = 0;
  for (const rc of validated) {
    for (const m of [...Object.values(rc.boys.marks), ...Object.values(rc.girls.marks)]) {
      if (m === "absent") absents += 1;
    }
  }
  const active = students.filter((s) => s.actif).length;
  const lines = [
    `Résumé internat (7 derniers appels validés) :`,
    `• ${active} interne(s) actif(s) sur ${rooms.length} chambre(s).`,
    `• ${validated.length} appel(s) du soir validé(s) sur la période.`,
    `• ${absents} marquage(s) « absent » enregistré(s).`,
  ];
  if (overCapacity.length > 0) {
    lines.push(`• Attention : ${overCapacity.length} chambre(s) en surbooking.`);
  } else {
    lines.push(`• Aucune chambre en surbooking.`);
  }
  return lines.join("\n");
}

export function buildDashboardStats(params: {
  students: InternatStudent[];
  rooms: InternatRoom[];
  tonightRollCall: InternatRollCall;
  recentRollCalls: InternatRollCall[];
  incidents?: InternatIncident[];
  weeklySummaryEnabled?: boolean;
}): InternatDashboardStats {
  const { students, rooms, tonightRollCall, recentRollCalls, incidents = [] } = params;
  const active = students.filter((s) => s.actif);
  const overCapacity = rooms
    .map((room) => {
      const count = active.filter((s) => s.roomId === room.id).length;
      return count > room.capacity ? { roomId: room.id, label: room.label, count, capacity: room.capacity } : null;
    })
    .filter(Boolean) as InternatDashboardStats["roomsOverCapacity"];

  const allMarks = { ...tonightRollCall.boys.marks, ...tonightRollCall.girls.marks };
  const tonightCounts = countMarks(allMarks);

  const boysComplete = sectionIsComplete(tonightRollCall.boys, students, "M");
  const girlsComplete = sectionIsComplete(tonightRollCall.girls, students, "F");

  let tonightStatus: InternatDashboardStats["tonightRollCall"]["status"] = "non_demarre";
  if (tonightRollCall.status === "validee") tonightStatus = "validee";
  else if (Object.keys(allMarks).length > 0 || tonightRollCall.boys.completed || tonightRollCall.girls.completed) {
    tonightStatus = "en_cours";
  }

  const lastValidated = recentRollCalls.find((r) => r.status === "validee");

  const cutoff = new Date();
  cutoff.setDate(cutoff.getDate() - 30);
  const cutoffStr = `${cutoff.getFullYear()}-${pad2(cutoff.getMonth() + 1)}-${pad2(cutoff.getDate())}`;
  const recentIncidents = incidents.filter((i) => i.occurredAt >= cutoffStr);
  const incidents30d = {
    incident: recentIncidents.filter((i) => i.kind === "incident").length,
    remarque: recentIncidents.filter((i) => i.kind === "remarque").length,
    sanction: recentIncidents.filter((i) => i.kind === "sanction").length,
    valorisation: recentIncidents.filter((i) => i.kind === "valorisation").length,
    total: recentIncidents.length,
  };

  const studentsUnderWatch = students
    .filter((s) => s.actif && s.underWatch)
    .map((s) => ({
      id: s.id,
      name: studentDisplayName(s),
      classe: s.classe,
      note: s.underWatchNote,
    }));

  const totalBeds = rooms.reduce((s, r) => s + r.capacity, 0);
  const occupiedBeds = active.filter((s) => s.roomId).length;
  const roomInsights = buildRoomInsights(rooms, students, incidents);
  let roomsFull = 0;
  let roomsEmpty = 0;
  let roomsPartial = 0;
  for (const ri of roomInsights) {
    if (ri.status === "empty") roomsEmpty += 1;
    else if (ri.status === "full" || ri.status === "over") roomsFull += 1;
    else roomsPartial += 1;
  }

  const stats: InternatDashboardStats = {
    activeStudents: active.length,
    students: {
      boys: active.filter((s) => s.sexe === "M").length,
      girls: active.filter((s) => s.sexe === "F").length,
      college: active.filter((s) => s.etablissement === "Collège").length,
      lycee: active.filter((s) => s.etablissement === "Lycée").length,
      withoutRoom: active.filter((s) => !s.roomId).length,
    },
    roomCount: rooms.length,
    occupancy: {
      totalBeds,
      occupiedBeds,
      fillRate: totalBeds > 0 ? Math.round((occupiedBeds / totalBeds) * 100) : null,
      roomsFull,
      roomsEmpty,
      roomsPartial,
    },
    wingOccupancy: buildWingOccupancy(rooms, students),
    roomsOverCapacity: overCapacity,
    roomsWithFreeSlots: rooms.filter((room) => {
      const count = active.filter((s) => s.roomId === room.id).length;
      return count < room.capacity;
    }).length,
    problematicRoomCount: roomInsights.filter((r) => r.isProblematic).length,
    tonightRollCall: {
      date: tonightRollCall.date,
      status: tonightStatus,
      presentCount: tonightCounts.present,
      absentCount: tonightCounts.absent,
      excusedCount: tonightCounts.excused,
      activityCount: tonightCounts.activity,
      boysComplete,
      girlsComplete,
    },
    presenceRate7d: computePresenceRate7d(recentRollCalls, students),
    incidents30d,
    studentsUnderWatch,
    lastValidatedRollCall: lastValidated
      ? {
          date: lastValidated.date,
          validatedBy: lastValidated.validatedBy,
          validatedAt: lastValidated.validatedAt,
        }
      : undefined,
  };

  if (params.weeklySummaryEnabled !== false) {
    stats.weeklySummary = buildWeeklySummary(recentRollCalls, students, rooms, overCapacity);
  }

  return stats;
}

export function rollCallStudentsByMark(
  rollCall: InternatRollCall,
  students: InternatStudent[],
  target: InternatRollMark | InternatRollMark[],
) {
  const targets = Array.isArray(target) ? target : [target];
  const out: Array<{ student: InternatStudent; mark: InternatRollMark }> = [];
  for (const s of students.filter((x) => x.actif)) {
    const mark = s.sexe === "M" ? rollCall.boys.marks[s.id] : rollCall.girls.marks[s.id];
    if (mark && targets.includes(mark)) {
      out.push({ student: s, mark });
    }
  }
  return out;
}

export function rollCallAbsentStudents(rollCall: InternatRollCall, students: InternatStudent[]) {
  return rollCallStudentsByMark(rollCall, students, ["absent", "excuse"]);
}

function formatStudentList(items: InternatStudent[]) {
  return items.map((s) => `${studentDisplayName(s)} (${s.classe}, ${s.etablissement})`).join("\n");
}
