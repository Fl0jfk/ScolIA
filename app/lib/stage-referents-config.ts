import { listStageEnabledClassNames } from "@/app/lib/stage-periods-config";
import { getJson, putJson } from "@/app/lib/s3-storage";
import { STAGE_S3, currentStageSchoolYear, type StageConvention } from "@/app/lib/stage-types";

export type StageReferentRole = "professeur_principal" | "professeur_referent";

export type StageClassReferentAssignment = {
  className: string;
  externalUserId: string;
  name: string;
  email: string;
  /** PP de la classe vs référent stage (peut être la même personne). */
  role: StageReferentRole;
};

export type StageReferentsConfig = {
  schoolYear: string;
  updatedAt: string;
  updatedBy?: string;
  assignments: StageClassReferentAssignment[];
};

function normalizeClassName(className: string): string {
  return className.trim();
}

export function classKey(className: string): string {
  return normalizeClassName(className)
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[°º]/g, "")
    .replace(/[\s._\-/]+/g, "")
    .toLowerCase();
}

function normalizeRole(raw: unknown): StageReferentRole {
  return raw === "professeur_principal" ? "professeur_principal" : "professeur_referent";
}

export async function listStageReferentClassNames(schoolYear?: string): Promise<string[]> {
  return listStageEnabledClassNames(schoolYear);
}

export async function getStageReferentsConfig(schoolYear: string): Promise<StageReferentsConfig | null> {
  const hit = await getJson<StageReferentsConfig>(STAGE_S3.referentsConfig(schoolYear));
  if (!hit?.data?.schoolYear) return null;
  return {
    schoolYear: hit.data.schoolYear,
    updatedAt: hit.data.updatedAt,
    updatedBy: hit.data.updatedBy,
    assignments: Array.isArray(hit.data.assignments)
      ? hit.data.assignments
          .map((a) => ({
            className: normalizeClassName(String(a.className ?? "")),
            externalUserId: String(a.externalUserId ?? "").trim(),
            name: String(a.name ?? "").trim(),
            email: String(a.email ?? "").trim().toLowerCase(),
            role: normalizeRole((a as { role?: unknown }).role),
          }))
          .filter((a) => a.className && a.externalUserId && a.email)
      : [],
  };
}

export async function saveStageReferentsConfig(
  config: StageReferentsConfig,
): Promise<StageReferentsConfig> {
  const next: StageReferentsConfig = {
    schoolYear: config.schoolYear,
    updatedAt: new Date().toISOString(),
    updatedBy: config.updatedBy,
    assignments: config.assignments
      .map((a) => ({
        className: normalizeClassName(a.className),
        externalUserId: a.externalUserId.trim(),
        name: a.name.trim(),
        email: a.email.trim().toLowerCase(),
        role: normalizeRole(a.role),
      }))
      .filter((a) => a.className && a.externalUserId && a.name && a.email),
  };
  await putJson(STAGE_S3.referentsConfig(next.schoolYear), next);
  return next;
}

export function findReferentAssignment(
  config: StageReferentsConfig | null | undefined,
  className: string,
): StageClassReferentAssignment | null {
  const all = findReferentAssignments(config, className);
  return (
    all.find((a) => a.role === "professeur_referent") ??
    all.find((a) => a.role === "professeur_principal") ??
    all[0] ??
    null
  );
}

export function findReferentAssignments(
  config: StageReferentsConfig | null | undefined,
  className: string,
): StageClassReferentAssignment[] {
  if (!config || !className.trim()) return [];
  const key = classKey(className);
  return config.assignments.filter((a) => classKey(a.className) === key);
}

export function findPrincipalAssignments(
  config: StageReferentsConfig | null | undefined,
  className: string,
): StageClassReferentAssignment[] {
  return findReferentAssignments(config, className).filter(
    (a) => a.role === "professeur_principal",
  );
}

async function resolveReferentForClass(
  className: string,
  schoolYear?: string,
): Promise<StageClassReferentAssignment | null> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStageReferentsConfig(year);
  return findReferentAssignment(config, className);
}

/** Classes dont l'utilisateur est professeur référent / principal. */
export async function listClassesForReferentUser(
  externalUserId: string,
  schoolYear?: string,
): Promise<string[]> {
  const id = externalUserId.trim();
  if (!id) return [];
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStageReferentsConfig(year);
  if (!config) return [];
  return [...new Set(
    config.assignments
      .filter((a) => a.externalUserId === id)
      .map((a) => a.className),
  )].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

/** True si l'utilisateur est PP (ou seul assigné historique) sur la classe. */
export async function userCanAssignStageReferentForClass(
  externalUserId: string,
  className: string,
  schoolYear?: string,
): Promise<boolean> {
  const id = externalUserId.trim();
  if (!id || !className.trim()) return false;
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStageReferentsConfig(year);
  const forClass = findReferentAssignments(config, className);
  if (forClass.length === 0) return false;
  const asPrincipal = forClass.some(
    (a) => a.externalUserId === id && a.role === "professeur_principal",
  );
  if (asPrincipal) return true;
  // Compat : anciennes configs sans PP explicite → tout assigné de la classe peut déléguer.
  const hasExplicitPrincipal = forClass.some((a) => a.role === "professeur_principal");
  if (!hasExplicitPrincipal) {
    return forClass.some((a) => a.externalUserId === id);
  }
  return false;
}

export async function ensureConventionReferent(convention: StageConvention): Promise<StageConvention> {
  const hasReferent =
    convention.teacherReferent.name.trim() && convention.teacherReferent.email.trim();
  if (hasReferent) return convention;

  const assignment = await resolveReferentForClass(
    convention.student.className,
    convention.schoolYear,
  );
  if (!assignment) return convention;

  return {
    ...convention,
    teacherReferent: {
      name: assignment.name,
      email: assignment.email,
      userId: assignment.externalUserId,
    },
  };
}
