import "server-only";

import { listDirectoryMembers } from "@/app/lib/directory-members";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";
import { hasRole } from "@/app/lib/intranet-role-utils";
import { readRhPlanning } from "@/app/lib/rh/planning-storage";
import type { TeacherPlanningDoc } from "@/app/lib/rh/planning-types";

export type TeacherPlanningEntry = {
  personnelId: string;
  displayName: string;
  planning: TeacherPlanningDoc;
};

const CACHE_TTL_MS = 90_000;

let cache: { entries: TeacherPlanningEntry[]; expiresAt: number } | null = null;

function teacherPlanningHasContent(doc: TeacherPlanningDoc): boolean {
  return (
    doc.weekA.length > 0 ||
    doc.weekB.length > 0 ||
    (doc.replacements?.length ?? 0) > 0
  );
}

async function loadTeacherPlanningEntriesFresh(): Promise<TeacherPlanningEntry[]> {
  const members = await listDirectoryMembers();
  const teachers = members.filter(
    (m) => m.externalUserId && !m.pending && hasRole(normalizeIntranetRoles(m.roles), "professeur"),
  );
  const nameById = new Map(
    teachers.map((m) => [m.externalUserId, m.displayName || m.email] as const),
  );

  const { listAllTeacherPlanningsForEtab } = await import("@/app/lib/rh/planning-storage");
  const fromDb = await listAllTeacherPlanningsForEtab();
  if (fromDb.length > 0) {
    return fromDb
      .filter((row) => teacherPlanningHasContent(row.planning))
      .map((row) => ({
        personnelId: row.personnelId,
        displayName: nameById.get(row.personnelId) || row.personnelId,
        planning: row.planning,
      }));
  }

  const entries = await Promise.all(
    teachers.map(async (m) => {
      const planning = await readRhPlanning("teacher", m.externalUserId);
      if (planning.kind !== "teacher" || !teacherPlanningHasContent(planning)) {
        return null;
      }
      return {
        personnelId: m.externalUserId,
        displayName: m.displayName || m.email,
        planning,
      };
    }),
  );

  return entries.filter((e): e is TeacherPlanningEntry => !!e);
}

/** Index EDT profs avec cache court (évite N×S3 par requête dossier / classe). */
export async function getTeacherPlanningEntries(force = false): Promise<TeacherPlanningEntry[]> {
  if (!force && cache && Date.now() < cache.expiresAt) {
    return cache.entries;
  }
  const entries = await loadTeacherPlanningEntriesFresh();
  cache = { entries, expiresAt: Date.now() + CACHE_TTL_MS };
  return entries;
}

export function invalidateTeacherPlanningIndex(): void {
  cache = null;
}
