import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { listClassesForTeacherUser } from "@/app/lib/class-allocation-teachers";
import { loadSchoolRoster } from "@/app/lib/school-roster";
import { getJson } from "@/app/lib/s3-storage";
import type { TeacherPlanningCatalog } from "@/app/lib/rh/planning-types";

export type { TeacherPlanningCatalog };

const ROOMS_KEY = "reservation-rooms/rooms.json";

function uniqueSorted(values: string[]): string[] {
  return [...new Set(values.map((v) => v.trim()).filter(Boolean))].sort((a, b) =>
    a.localeCompare(b, "fr", { sensitivity: "base" }),
  );
}

function roomLabel(raw: unknown): string | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const name = typeof o.name === "string" ? o.name.trim() : "";
  const label = typeof o.label === "string" ? o.label.trim() : "";
  return name || label || null;
}

async function loadRoomLabels(): Promise<string[]> {
  try {
    const hit = await getJson<{ rooms?: unknown[] } | unknown[]>(ROOMS_KEY);
    const data = hit?.data;
    const rows = Array.isArray(data) ? data : (data as { rooms?: unknown[] })?.rooms || [];
    return uniqueSorted(rows.map(roomLabel).filter((r): r is string => !!r));
  } catch {
    return [];
  }
}

/** Catalogue EDT pour saisie manuelle (matières, classes, salles). */
export async function loadTeacherPlanningCatalog(
  externalUserId: string,
): Promise<TeacherPlanningCatalog> {
  const [appCfg, roster, assignedClasses, rooms] = await Promise.all([
    loadAppConfig(),
    loadSchoolRoster(),
    listClassesForTeacherUser(externalUserId),
    loadRoomLabels(),
  ]);

  const subjects = uniqueSorted(Object.keys(appCfg.profRoom.subjectColors || {}));

  const classesFromProfRoom = Object.values(appCfg.profRoom.classesByPole || {}).flat();
  const classesFromRoster = roster.classAssignments.map((a) => a.className);
  const classes = uniqueSorted([
    ...classesFromProfRoom,
    ...classesFromRoster,
    ...assignedClasses,
  ]);

  return {
    subjects,
    classes,
    rooms,
    assignedClasses,
  };
}
