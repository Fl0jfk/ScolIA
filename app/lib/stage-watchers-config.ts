import { listStageEnabledClassNames } from "@/app/lib/stage-periods-config";
import { classKey } from "@/app/lib/stage-referents-config";
import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  STAGE_S3,
  currentStageSchoolYear,
  type StageConvention,
} from "@/app/lib/stage-types";

/** CPE = visu stages ; restauration = jours d'absence repas. */
export type StageWatcherKind = "cpe" | "restauration";

export type StageClassWatcherAssignment = {
  scope: "class";
  className: string;
  externalUserId: string;
  name: string;
  email: string;
  kind: StageWatcherKind;
};

export type StageStudentWatcherAssignment = {
  scope: "student";
  /** Clé dossier élève (nom|prénom|classe) ou libellé libre. */
  studentKey: string;
  studentFirstName: string;
  studentLastName: string;
  studentClassName: string;
  externalUserId: string;
  name: string;
  email: string;
  kind: StageWatcherKind;
};

export type StageWatcherAssignment = StageClassWatcherAssignment | StageStudentWatcherAssignment;

export type StageWatchersConfig = {
  schoolYear: string;
  updatedAt: string;
  updatedBy?: string;
  assignments: StageWatcherAssignment[];
};

function normalizeKind(raw: unknown): StageWatcherKind {
  return raw === "restauration" ? "restauration" : "cpe";
}

function studentKeyOf(params: {
  firstName: string;
  lastName: string;
  className: string;
}): string {
  return `${params.lastName.trim().toLowerCase()}|${params.firstName.trim().toLowerCase()}|${params.className.trim().toLowerCase()}`;
}

export async function getStageWatchersConfig(schoolYear: string): Promise<StageWatchersConfig | null> {
  const hit = await getJson<StageWatchersConfig>(STAGE_S3.watchersConfig(schoolYear));
  if (!hit?.data?.schoolYear) return null;
  const assignments: StageWatcherAssignment[] = [];
  for (const raw of Array.isArray(hit.data.assignments) ? hit.data.assignments : []) {
    if (!raw || typeof raw !== "object") continue;
    const a = raw as Record<string, unknown>;
    const externalUserId = String(a.externalUserId ?? "").trim();
    const name = String(a.name ?? "").trim();
    const email = String(a.email ?? "").trim().toLowerCase();
    const kind = normalizeKind(a.kind);
    if (!externalUserId || !name || !email) continue;
    if (a.scope === "student") {
      const studentFirstName = String(a.studentFirstName ?? "").trim();
      const studentLastName = String(a.studentLastName ?? "").trim();
      const studentClassName = String(a.studentClassName ?? "").trim();
      if (!studentFirstName || !studentLastName) continue;
      assignments.push({
        scope: "student",
        studentKey:
          String(a.studentKey ?? "").trim() ||
          studentKeyOf({
            firstName: studentFirstName,
            lastName: studentLastName,
            className: studentClassName,
          }),
        studentFirstName,
        studentLastName,
        studentClassName,
        externalUserId,
        name,
        email,
        kind,
      });
    } else {
      const className = String(a.className ?? "").trim();
      if (!className) continue;
      assignments.push({
        scope: "class",
        className,
        externalUserId,
        name,
        email,
        kind,
      });
    }
  }
  return {
    schoolYear: hit.data.schoolYear,
    updatedAt: hit.data.updatedAt,
    updatedBy: hit.data.updatedBy,
    assignments,
  };
}

export async function saveStageWatchersConfig(
  config: StageWatchersConfig,
): Promise<StageWatchersConfig> {
  const next: StageWatchersConfig = {
    schoolYear: config.schoolYear,
    updatedAt: new Date().toISOString(),
    updatedBy: config.updatedBy,
    assignments: config.assignments,
  };
  await putJson(STAGE_S3.watchersConfig(next.schoolYear), next);
  return next;
}

export async function listStageWatcherClassNames(schoolYear?: string): Promise<string[]> {
  return listStageEnabledClassNames(schoolYear);
}

export function listWatcherAssignmentsForUser(
  config: StageWatchersConfig | null | undefined,
  externalUserId: string,
  kind?: StageWatcherKind,
): StageWatcherAssignment[] {
  const id = externalUserId.trim();
  if (!config || !id) return [];
  return config.assignments.filter(
    (a) => a.externalUserId === id && (kind ? a.kind === kind : true),
  );
}

export function conventionMatchesWatcherAssignments(
  convention: StageConvention,
  assignments: StageWatcherAssignment[],
): boolean {
  if (assignments.length === 0) return false;
  const classK = classKey(convention.student.className);
  const sk = studentKeyOf({
    firstName: convention.student.firstName,
    lastName: convention.student.lastName,
    className: convention.student.className,
  });
  for (const a of assignments) {
    if (a.scope === "class" && classKey(a.className) === classK) return true;
    if (a.scope === "student") {
      if (a.studentKey === sk) return true;
      if (
        a.studentLastName.trim().toLowerCase() === convention.student.lastName.trim().toLowerCase() &&
        a.studentFirstName.trim().toLowerCase() ===
          convention.student.firstName.trim().toLowerCase()
      ) {
        return true;
      }
    }
  }
  return false;
}

export async function listClassesForCpeWatcherUser(
  externalUserId: string,
  schoolYear?: string,
): Promise<string[]> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStageWatchersConfig(year);
  return [
    ...new Set(
      listWatcherAssignmentsForUser(config, externalUserId, "cpe")
        .filter((a): a is StageClassWatcherAssignment => a.scope === "class")
        .map((a) => a.className),
    ),
  ].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

export async function userHasStageWatcherAccess(
  externalUserId: string,
  schoolYear?: string,
): Promise<boolean> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStageWatchersConfig(year);
  return listWatcherAssignmentsForUser(config, externalUserId).length > 0;
}
