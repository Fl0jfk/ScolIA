import "server-only";

import { classKey } from "@/app/lib/stage-referents-config";
import {
  listClassesForReferentUser,
} from "@/app/lib/stage-referents-config";
import { readRhPlanning } from "@/app/lib/rh/planning-storage";
import { currentStageSchoolYear } from "@/app/lib/stage-types";
import { hasGlobalAdminRole } from "@/app/lib/intranet-role-utils";
import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import {
  loadSchoolRoster,
  type SchoolRosterConfig,
} from "@/app/lib/school-roster";
import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";

export type ClassAllocationTeacherAssignment = {
  className: string;
  externalUserId: string;
  name: string;
  email: string;
};

function studentMatchesClass(studentClasse: string | undefined, className: string): boolean {
  if (schoolClassesMatch(studentClasse, className)) return true;
  // Repli historique classKey (stages) — préfixe prudent uniquement.
  const student = classKey(String(studentClasse ?? ""));
  const target = classKey(className);
  if (!student || !target) return false;
  if (student === target) return true;
  if (student.length >= 3 && target.length >= 3 && (student.startsWith(target) || target.startsWith(student))) {
    return true;
  }
  return false;
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
    roles.includes("surveillant") ||
    roles.includes("cpe")
  );
}

/** Affectations prof ↔ classe depuis le référentiel global (paramètres). */
async function loadTeacherAssignmentsFromRoster(): Promise<ClassAllocationTeacherAssignment[]> {
  const roster = await loadSchoolRoster();
  return roster.classAssignments;
}

async function listClassesFromTeacherEdt(externalUserId: string): Promise<string[]> {
  const doc = await readRhPlanning("teacher", externalUserId);
  if (doc.kind !== "teacher") return [];
  const set = new Set<string>();
  for (const slot of [...doc.weekA, ...doc.weekB]) {
    for (const c of slot.classes || []) {
      const t = c.trim();
      if (t) set.add(t);
    }
  }
  for (const r of doc.replacements || []) {
    for (const c of r.classes || []) {
      const t = c.trim();
      if (t) set.add(t);
    }
  }
  return [...set].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

export async function listClassesForTeacherUser(
  externalUserId: string,
  _campaignId?: string,
): Promise<string[]> {
  const roster = await loadSchoolRoster();
  const fromRoster = roster.classAssignments
    .filter((a) => a.externalUserId === externalUserId)
    .map((a) => a.className);
  const fromReferents = await listClassesForReferentUser(
    externalUserId,
    currentStageSchoolYear(),
  );
  const fromEdt = await listClassesFromTeacherEdt(externalUserId);
  // Union roster Paramètres + référents Stages + classes EDT (P2).
  const merged = [...new Set([...fromRoster, ...fromReferents, ...fromEdt])];
  return merged.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

function teacherCatalogFromRoster(roster: SchoolRosterConfig): string[] {
  return roster.teacherCatalog;
}
