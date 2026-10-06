import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";
import { notifyStageReferentStudentAssignments } from "@/app/lib/stage-notify";
import {
  classKey,
  findStudentReferentAssignments,
  getStageReferentsConfig,
  saveStageReferentsConfig,
  stageRosterStudentKey,
  type StageStudentReferentAssignment,
} from "@/app/lib/stage-referents-config";
import { loadStageConventionsByIds } from "@/app/lib/stage-convention-load";
import { getConventionsIndex, invalidateStageClassRosterCaches, saveStageConvention } from "@/app/lib/stage-storage";
import { currentStageSchoolYear } from "@/app/lib/stage-types";

function normalizePersonName(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

function namesMatchStudent(
  assignment: StageStudentReferentAssignment,
  lastName: string,
  firstName: string,
): boolean {
  const keyFromNames = stageRosterStudentKey(lastName, firstName, assignment.ine);
  if (assignment.studentKey === keyFromNames) return true;
  if (assignment.studentKey === stageRosterStudentKey(lastName, firstName)) return true;
  const blob = normalizePersonName(assignment.studentName);
  return blob.includes(normalizePersonName(lastName)) && blob.includes(normalizePersonName(firstName));
}

export async function upsertStudentReferentAssignments(params: {
  className: string;
  schoolYear?: string;
  updatedBy?: string;
  teacher: { externalUserId: string; name: string; email: string };
  students: Array<{
    key: string;
    eleveId?: string;
    ine?: string;
    studentName: string;
  }>;
}): Promise<{
  config: Awaited<ReturnType<typeof saveStageReferentsConfig>>;
  updatedConventions: number;
}> {
  const className = params.className.trim();
  const year = params.schoolYear?.trim() || currentStageSchoolYear();
  const teacher = {
    externalUserId: params.teacher.externalUserId.trim(),
    name: params.teacher.name.trim(),
    email: params.teacher.email.trim().toLowerCase(),
  };
  if (!className || !teacher.externalUserId || !teacher.name || !teacher.email) {
    throw new Error("Professeur référent incomplet.");
  }

  const incoming: StageStudentReferentAssignment[] = params.students
    .map((s) => ({
      className,
      studentKey: s.key.trim(),
      eleveId: s.eleveId?.trim() || undefined,
      ine: s.ine?.trim() || undefined,
      studentName: s.studentName.trim(),
      externalUserId: teacher.externalUserId,
      name: teacher.name,
      email: teacher.email,
    }))
    .filter((s) => s.studentKey && s.studentName);

  if (incoming.length === 0) {
    throw new Error("Sélectionnez au moins un élève.");
  }

  const existing = await getStageReferentsConfig(year);
  if (!existing) {
    throw new Error("Configurez d'abord les professeurs de la classe dans Stages → Réglages.");
  }
  const kept = existing.studentAssignments.filter((row) => {
    if (classKey(row.className) !== classKey(className)) return true;
    return !incoming.some((s) => s.studentKey === row.studentKey);
  });

  const config = await saveStageReferentsConfig({
    schoolYear: year,
    updatedAt: new Date().toISOString(),
    updatedBy: params.updatedBy,
    assignments: existing.assignments,
    studentAssignments: [...kept, ...incoming],
  });

  const updatedConventions = await applyStudentReferentsToConventions({
    className,
    schoolYear: year,
    assignments: incoming,
    byName: params.updatedBy || "Administratif",
  });

  await invalidateStageClassRosterCaches();
  return { config, updatedConventions };
}

async function applyStudentReferentsToConventions(params: {
  className: string;
  schoolYear: string;
  assignments: StageStudentReferentAssignment[];
  byName: string;
}): Promise<number> {
  const index = await getConventionsIndex();
  const candidateIds = index
    .filter((e) => schoolClassesMatch(String(e.className ?? ""), params.className))
    .map((e) => e.id);
  if (candidateIds.length === 0) return 0;

  const conventions = await loadStageConventionsByIds(candidateIds);
  const now = new Date().toISOString();
  let count = 0;

  for (const convention of conventions) {
    if (convention.status === "archived" || convention.status === "cancelled") continue;
    const hit = params.assignments.find((a) =>
      namesMatchStudent(a, convention.student.lastName, convention.student.firstName),
    );
    if (!hit) continue;
    const same =
      convention.teacherReferent.email?.trim().toLowerCase() === hit.email &&
      convention.teacherReferent.userId === hit.externalUserId;
    if (same) continue;

    await saveStageConvention({
      ...convention,
      teacherReferent: {
        name: hit.name,
        email: hit.email,
        userId: hit.externalUserId,
      },
      updatedAt: now,
      history: [
        ...convention.history,
        {
          at: now,
          by: params.byName,
          action: "REFERENT_DELEGUE",
          note: `${hit.name} <${hit.email}>`,
        },
      ],
    });
    count += 1;
  }

  return count;
}

export async function notifyReferentAssignmentsForClass(params: {
  className: string;
  schoolYear?: string;
  teacherExternalUserId?: string;
}): Promise<{
  sent: number;
  skipped: number;
  results: Array<{ email: string; name: string; sent: boolean; reason?: string; studentCount: number }>;
}> {
  const className = params.className.trim();
  const year = params.schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStageReferentsConfig(year);
  const rows = findStudentReferentAssignments(config, className);
  const byTeacher = new Map<string, StageStudentReferentAssignment[]>();
  for (const row of rows) {
    if (params.teacherExternalUserId && row.externalUserId !== params.teacherExternalUserId) {
      continue;
    }
    const list = byTeacher.get(row.externalUserId) ?? [];
    list.push(row);
    byTeacher.set(row.externalUserId, list);
  }

  const results: Array<{
    email: string;
    name: string;
    sent: boolean;
    reason?: string;
    studentCount: number;
  }> = [];

  for (const list of byTeacher.values()) {
    const first = list[0]!;
    const studentNames = [...new Set(list.map((r) => r.studentName))].sort((a, b) =>
      a.localeCompare(b, "fr", { sensitivity: "base" }),
    );
    const mail = await notifyStageReferentStudentAssignments({
      className,
      teacherName: first.name,
      teacherEmail: first.email,
      studentNames,
    });
    results.push({
      email: first.email,
      name: first.name,
      sent: mail.sent,
      reason: mail.sent ? undefined : mail.reason,
      studentCount: studentNames.length,
    });
  }

  return {
    sent: results.filter((r) => r.sent).length,
    skipped: results.filter((r) => !r.sent).length,
    results,
  };
}
