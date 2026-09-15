import "server-only";

import { loadAppConfig } from "@/app/lib/app-config";
import { listClassesForTeacherUser } from "@/app/lib/class-allocation-teachers";
import { resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import { listMatieres } from "@/app/lib/notes-config-db";
import { listNomenclatureByType } from "@/app/lib/ref-nomenclature-db";
import { loadSchoolRoster } from "@/app/lib/school-roster";
import { getJson } from "@/app/lib/s3-storage";
import type { TeacherPlanningCatalog } from "@/app/lib/rh/planning-types";
import { pickDefaultTimetableGrid } from "@/app/lib/rh/timetable-grids";

export type { TeacherPlanningCatalog };

const ROOMS_KEY = "reservation-rooms/rooms.json";

export const EMPTY_TEACHER_PLANNING_CATALOG: TeacherPlanningCatalog = {
  subjects: [],
  classes: [],
  rooms: [],
  assignedClasses: [],
  timetableGrid: null,
  teachingGroups: [],
};

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

/**
 * Matières académiques pour l’EDT : `note_matiere` (sync nomenclature),
 * sinon `ref_nomenclature` type=matiere. Ne jamais utiliser
 * `profRoom.subjectColors` (couleurs du module réservation de salles).
 */
async function loadAcademicSubjectLabels(): Promise<string[]> {
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return [];

  try {
    const matieres = await listMatieres(etabId);
    const fromNotes = uniqueSorted(
      matieres.filter((m) => m.actif !== false).map((m) => m.libelle || m.code),
    );
    if (fromNotes.length > 0) return fromNotes;

    const nomenc = await listNomenclatureByType(etabId, "matiere");
    return uniqueSorted(nomenc.map((e) => e.libelleLong || e.libelleCourt || e.code));
  } catch (error) {
    console.error("[rh/planning-catalog] loadAcademicSubjectLabels", error);
    return [];
  }
}

function settledValue<T>(result: PromiseSettledResult<T>, fallback: T): T {
  return result.status === "fulfilled" ? result.value : fallback;
}

/** Catalogue EDT pour saisie manuelle (matières, classes, salles). */
export async function loadTeacherPlanningCatalog(
  externalUserId: string,
): Promise<TeacherPlanningCatalog> {
  try {
    const [appCfgRes, rosterRes, assignedRes, roomsRes, subjectsRes] = await Promise.allSettled([
      loadAppConfig(),
      loadSchoolRoster(),
      listClassesForTeacherUser(externalUserId),
      loadRoomLabels(),
      loadAcademicSubjectLabels(),
    ]);

    for (const [label, res] of [
      ["appConfig", appCfgRes],
      ["roster", rosterRes],
      ["assignedClasses", assignedRes],
      ["rooms", roomsRes],
      ["subjects", subjectsRes],
    ] as const) {
      if (res.status === "rejected") {
        console.error(`[rh/planning-catalog] ${label}`, res.reason);
      }
    }

    const appCfg = settledValue(appCfgRes, null);
    const roster = settledValue(rosterRes, null);
    const assignedClasses = settledValue(assignedRes, [] as string[]);
    const rooms = settledValue(roomsRes, [] as string[]);
    const subjects = settledValue(subjectsRes, [] as string[]);

    const classesFromProfRoom = Object.values(appCfg?.profRoom.classesByPole || {}).flat();
    const classesFromRoster = (roster?.classAssignments || []).map((a) => a.className);
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
      timetableGrid: appCfg ? pickDefaultTimetableGrid(appCfg.timetableGrids) : null,
      teachingGroups: appCfg?.teachingGroups?.groups ?? [],
    };
  } catch (error) {
    console.error("[rh/planning-catalog] loadTeacherPlanningCatalog", error);
    return { ...EMPTY_TEACHER_PLANNING_CATALOG };
  }
}

/** Comme `loadTeacherPlanningCatalog`, mais abandonne après `ms` pour ne pas bloquer l’UI. */
export async function loadTeacherPlanningCatalogWithTimeout(
  externalUserId: string,
  ms = 8000,
): Promise<TeacherPlanningCatalog> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      loadTeacherPlanningCatalog(externalUserId),
      new Promise<TeacherPlanningCatalog>((resolve) => {
        timer = setTimeout(() => {
          console.warn("[rh/planning-catalog] timeout catalogue EDT", ms);
          resolve({ ...EMPTY_TEACHER_PLANNING_CATALOG });
        }, ms);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}
