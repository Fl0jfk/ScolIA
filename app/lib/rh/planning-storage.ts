import "server-only";

import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  emptyStaffPlanning,
  emptyTeacherPlanning,
  normalizeStaffPlanning,
  normalizeTeacherPlanning,
  planningKeyFor,
  type RhPlanningDoc,
  type RhPlanningKind,
  type StaffPlanningDoc,
  type TeacherPlanningDoc,
} from "@/app/lib/rh/planning-types";

async function readTeacherPlanning(personnelId: string): Promise<TeacherPlanningDoc> {
  const hit = await getJson<unknown>(planningKeyFor("teacher", personnelId));
  if (!hit?.data) {
    return { ...emptyTeacherPlanning(personnelId), updatedAt: "" };
  }
  return normalizeTeacherPlanning(hit.data, personnelId);
}

async function readStaffPlanning(personnelId: string): Promise<StaffPlanningDoc> {
  const hit = await getJson<unknown>(planningKeyFor("staff", personnelId));
  if (!hit?.data) {
    return { ...emptyStaffPlanning(personnelId), updatedAt: "" };
  }
  return normalizeStaffPlanning(hit.data, personnelId);
}

export async function readRhPlanning(
  kind: RhPlanningKind,
  personnelId: string,
): Promise<RhPlanningDoc> {
  if (kind === "teacher") return readTeacherPlanning(personnelId);
  return readStaffPlanning(personnelId);
}

export async function writeRhPlanning(doc: RhPlanningDoc): Promise<RhPlanningDoc> {
  const key = planningKeyFor(doc.kind, doc.personnelId);
  const normalized =
    doc.kind === "teacher"
      ? normalizeTeacherPlanning(doc, doc.personnelId)
      : normalizeStaffPlanning(doc, doc.personnelId);
  const next = {
    ...normalized,
    updatedAt: new Date().toISOString(),
    source: normalized.source || (doc.source ?? "manual"),
    sourceFileName: normalized.sourceFileName || doc.sourceFileName,
  } as RhPlanningDoc;
  await putJson(key, next);
  return next;
}

function ensurePlanningShape(
  kind: RhPlanningKind,
  personnelId: string,
  updatedBy: string,
  staffMode: "fixed" | "rotation" = "fixed",
): RhPlanningDoc {
  return kind === "teacher"
    ? emptyTeacherPlanning(personnelId, updatedBy)
    : emptyStaffPlanning(personnelId, staffMode, updatedBy);
}
