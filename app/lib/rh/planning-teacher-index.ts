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
  const byId = new Map(fromDb.map((row) => [row.personnelId, row.planning] as const));

  const entries: TeacherPlanningEntry[] = [];
  for (const m of teachers) {
    const id = m.externalUserId;
    let planning = byId.get(id);
    if (!planning || !teacherPlanningHasContent(planning)) {
      const read = await readRhPlanning("teacher", id);
      if (read.kind === "teacher" && teacherPlanningHasContent(read)) {
        planning = read;
      }
    }
    if (planning && teacherPlanningHasContent(planning)) {
      entries.push({
        personnelId: id,
        displayName: nameById.get(id) || id,
        planning,
      });
    }
  }

  return entries.sort((a, b) => a.displayName.localeCompare(b.displayName, "fr"));
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
