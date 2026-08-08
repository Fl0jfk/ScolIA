import "server-only";

import { classKey } from "@/app/lib/stage-referents-config";
import {
  listClassesForReferentUser,
} from "@/app/lib/stage-referents-config";
import { currentStageSchoolYear } from "@/app/lib/stage-types";
import { hasGlobalAdminRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import {
  loadSchoolRoster,
  type SchoolRosterConfig,
} from "@/app/lib/school-roster";

export type ClassAllocationTeacherAssignment = {
  className: string;
  clerkUserId: string;
  name: string;
  email: string;
};

export function studentMatchesClass(studentClasse: string | undefined, className: string): boolean {
  const student = classKey(String(studentClasse ?? ""));
  const target = classKey(className);
  if (!student || !target) return false;
  return student === target || student.startsWith(target) || target.startsWith(student);
}

export function studentInAssignedClasses(
  studentClasse: string | undefined,
  assignedClasses: string[],
): boolean {
  if (!assignedClasses.length) return false;
  return assignedClasses.some((c) => studentMatchesClass(studentClasse, c));
}

export function canManageAllClassAllocationStudents(roles: string[]): boolean {
  if (hasGlobalAdminRole(roles)) return true;
  return (
    roles.some((r) => INTRANET_DIRECTION_SLUGS.includes(r as (typeof INTRANET_DIRECTION_SLUGS)[number])) ||
    roles.includes("administratif") ||
    roles.includes("education") ||
    roles.includes("cpe")
  );
}

/** Affectations prof ↔ classe depuis le référentiel global (paramètres). */
export async function loadTeacherAssignmentsFromRoster(): Promise<ClassAllocationTeacherAssignment[]> {
  const roster = await loadSchoolRoster();
  return roster.classAssignments;
}

export async function listClassesForTeacherUser(
  clerkUserId: string,
  _campaignId?: string,
): Promise<string[]> {
  const roster = await loadSchoolRoster();
  const fromRoster = roster.classAssignments
    .filter((a) => a.clerkUserId === clerkUserId)
    .map((a) => a.className);
  if (fromRoster.length) {
    return fromRoster.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
  }
  return listClassesForReferentUser(clerkUserId, currentStageSchoolYear());
}

export function teacherCatalogFromRoster(roster: SchoolRosterConfig): string[] {
  return roster.teacherCatalog;
}
