import "server-only";

import { listDirectoryMembers } from "@/app/lib/directory-members";
import {
  isEntCoreDbEnabled,
  loadSchoolRosterFromDb,
  replaceSchoolRosterInDb,
  resolveCurrentEtablissementId,
} from "@/app/lib/ent-core-db";
import { classKey } from "@/app/lib/stage-referents-config";
import {
  saveStageReferentsConfig,
  type StageClassReferentAssignment,
} from "@/app/lib/stage-referents-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";
import type { ClassAllocationTeacherAssignment } from "@/app/lib/class-allocation-teachers";

export type SchoolRosterConfig = {
  updatedAt: string;
  updatedBy?: string;
  /** Noms affichés dans les vœux parents (répartition des classes). */
  teacherCatalog: string[];
  /** Professeur principal / référent par classe — accès modules internes. */
  classAssignments: ClassAllocationTeacherAssignment[];
};

function defaultSchoolRoster(): SchoolRosterConfig {
  return {
    updatedAt: new Date().toISOString(),
    teacherCatalog: [],
    classAssignments: [],
  };
}

export async function loadSchoolRoster(): Promise<SchoolRosterConfig> {
  if (!isEntCoreDbEnabled()) {
    throw new Error("[school-roster] Postgres requis — plus de JSON");
  }
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) return defaultSchoolRoster();
  const fromDb = await loadSchoolRosterFromDb(etabId);
  return fromDb ?? defaultSchoolRoster();
}

async function syncStageReferentsFromRoster(config: SchoolRosterConfig): Promise<void> {
  const year = currentStageSchoolYear();
  const assignments: StageClassReferentAssignment[] = config.classAssignments.map((a) => ({
    className: a.className,
    externalUserId: a.externalUserId,
    name: a.name,
    email: a.email,
  }));
  await saveStageReferentsConfig({
    schoolYear: year,
    updatedAt: new Date().toISOString(),
    updatedBy: config.updatedBy,
    assignments,
  });
}

export async function saveSchoolRoster(
  config: Omit<SchoolRosterConfig, "updatedAt"> & { updatedBy?: string },
): Promise<SchoolRosterConfig> {
  const next: SchoolRosterConfig = {
    updatedAt: new Date().toISOString(),
    updatedBy: config.updatedBy,
    teacherCatalog: config.teacherCatalog.map((s) => s.trim()).filter(Boolean),
    classAssignments: config.classAssignments
      .map((a) => ({
        className: a.className.trim(),
        externalUserId: a.externalUserId.trim(),
        name: a.name.trim(),
        email: a.email.trim().toLowerCase(),
      }))
      .filter((a) => a.className && a.externalUserId && a.name && a.email),
  };
  if (!isEntCoreDbEnabled()) {
    throw new Error("[school-roster] Postgres requis — plus de JSON");
  }
  const etabId = await resolveCurrentEtablissementId();
  if (!etabId) throw new Error("[school-roster] établissement introuvable");
  await replaceSchoolRosterInDb(etabId, next);
  await syncStageReferentsFromRoster(next);
  return next;
}

async function listClassesForTeacherFromRoster(externalUserId: string): Promise<string[]> {
  const roster = await loadSchoolRoster();
  return roster.classAssignments
    .filter((a) => a.externalUserId === externalUserId)
    .map((a) => a.className)
    .sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

function assignmentForClass(
  roster: SchoolRosterConfig,
  className: string,
): ClassAllocationTeacherAssignment | null {
  const key = classKey(className);
  return roster.classAssignments.find((a) => classKey(a.className) === key) ?? null;
}

export async function importAssignmentsFromStageReferents(
  updatedBy?: string,
): Promise<SchoolRosterConfig> {
  const { getStageReferentsConfig } = await import("@/app/lib/stage-referents-config");
  const stage = await getStageReferentsConfig(currentStageSchoolYear());
  const current = await loadSchoolRoster();
  return saveSchoolRoster({
    teacherCatalog: current.teacherCatalog,
    classAssignments: (stage?.assignments ?? []).map((a) => ({
      className: a.className,
      externalUserId: a.externalUserId,
      name: a.name,
      email: a.email,
    })),
    updatedBy,
  });
}

/** Professeurs éligibles (prof / éducation). */
export async function listTeacherDirectoryOptions() {
  const members = await listDirectoryMembers();
  return members
    .filter((m) => m.externalUserId && !m.pending)
    .filter(
      (m) =>
        m.roles.includes("professeur") ||
        m.roles.includes("surveillant") ||
        m.roles.includes("cpe"),
    )
    .map((m) => ({
      externalUserId: m.externalUserId,
      email: m.email,
      displayName: m.displayName || m.email,
    }));
}
