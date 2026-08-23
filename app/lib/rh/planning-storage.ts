import "server-only";

import { getJson, putJson } from "@/app/lib/s3-storage";
import { getPersonnelIndex } from "@/app/lib/personnel-storage";
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
import {
  listTeacherPlanningsFromDb,
  readTeacherPlanningFromDb,
  resolveTeacherPlanningEtabId,
  writeTeacherPlanningToDb,
} from "@/app/lib/rh/planning-teacher-db";

function staffHasContent(doc: StaffPlanningDoc): boolean {
  if (doc.fixedSlots.length > 0) return true;
  if ((doc.exceptions?.length ?? 0) > 0) return true;
  return doc.rotations.some((r) => r.slots.length > 0);
}

function teacherHasContent(doc: TeacherPlanningDoc): boolean {
  return (
    doc.weekA.length > 0 ||
    doc.weekB.length > 0 ||
    (doc.replacements?.length ?? 0) > 0 ||
    Boolean(doc.updatedAt)
  );
}

async function readTeacherPlanningLegacy(personnelId: string): Promise<TeacherPlanningDoc> {
  const hit = await getJson<unknown>(planningKeyFor("teacher", personnelId));
  if (!hit?.data) {
    return { ...emptyTeacherPlanning(personnelId), updatedAt: "" };
  }
  return normalizeTeacherPlanning(hit.data, personnelId);
}

async function readTeacherPlanning(personnelId: string): Promise<TeacherPlanningDoc> {
  const etabId = await resolveTeacherPlanningEtabId();
  if (etabId) {
    try {
      const fromDb = await readTeacherPlanningFromDb(etabId, personnelId);
      if (fromDb && teacherHasContent(fromDb)) return fromDb;
    } catch (error) {
      console.error("[planning] lecture teacher_planning", error);
    }
  }
  return readTeacherPlanningLegacy(personnelId);
}

async function readStaffPlanning(personnelId: string): Promise<StaffPlanningDoc> {
  const hit = await getJson<unknown>(planningKeyFor("staff", personnelId));
  if (!hit?.data) {
    return { ...emptyStaffPlanning(personnelId), updatedAt: "" };
  }
  return normalizeStaffPlanning(hit.data, personnelId);
}

/**
 * Planning OGEC identifié par l’utilisateur (stockage Scaleway / collection).
 * Repli : ancienne clé dossier RH, si un JSON existait déjà.
 */
export async function readStaffPlanningForExternalUser(externalUserId: string): Promise<StaffPlanningDoc> {
  const id = externalUserId.trim();
  if (!id) return { ...emptyStaffPlanning(""), updatedAt: "" };

  const primary = await readStaffPlanning(id);
  if (primary.updatedAt || staffHasContent(primary)) {
    return { ...primary, personnelId: id };
  }

  try {
    const index = await getPersonnelIndex();
    const self = index.find((e) => e.externalUserId === id && e.active !== false);
    if (self?.id && self.id !== id) {
      const legacy = await readStaffPlanning(self.id);
      if (legacy.updatedAt || staffHasContent(legacy)) {
        return { ...legacy, personnelId: id };
      }
    }
  } catch {
    // Index RH optionnel — le planning ne dépend plus d’une fiche / OneDrive RH.
  }

  return { ...emptyStaffPlanning(id), updatedAt: "" };
}

export async function readRhPlanning(
  kind: RhPlanningKind,
  personnelId: string,
): Promise<RhPlanningDoc> {
  if (kind === "teacher") return readTeacherPlanning(personnelId);
  return readStaffPlanningForExternalUser(personnelId);
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

  if (next.kind === "teacher") {
    const etabId = await resolveTeacherPlanningEtabId();
    if (etabId) {
      const saved = await writeTeacherPlanningToDb(etabId, next);
      // Miroir collection JSON pour compat lecture legacy / outils.
      try {
        await putJson(key, saved);
      } catch (error) {
        console.error("[planning] miroir JSON teacher", error);
      }
      return saved;
    }
  }

  await putJson(key, next);
  return next;
}

/** Charge tous les EDT profs : tables relationnelles + repli collections JSON. */
export async function listAllTeacherPlanningsForEtab(): Promise<
  { personnelId: string; planning: TeacherPlanningDoc }[]
> {
  const etabId = await resolveTeacherPlanningEtabId();
  const byId = new Map<string, TeacherPlanningDoc>();

  if (etabId) {
    try {
      for (const doc of await listTeacherPlanningsFromDb(etabId)) {
        byId.set(doc.personnelId, doc);
      }
    } catch (error) {
      console.error("[planning] listTeacherPlanningsFromDb", error);
    }
  }

  return [...byId.entries()].map(([personnelId, planning]) => ({ personnelId, planning }));
}
