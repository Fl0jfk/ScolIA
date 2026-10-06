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

export type StageStudentReferentAssignment = {
  className: string;
  studentKey: string;
  eleveId?: string;
  ine?: string;
  studentName: string;
  externalUserId: string;
  name: string;
  email: string;
};

export type StageReferentsConfig = {
  schoolYear: string;
  updatedAt: string;
  updatedBy?: string;
  assignments: StageClassReferentAssignment[];
  /** Élève → professeur référent (suivi), indépendant de la signature PP. */
  studentAssignments: StageStudentReferentAssignment[];
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

function normalizePersonName(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

export function stageRosterStudentKey(nom: string, prenom: string, ine?: string): string {
  if (ine?.trim()) return `ine:${ine.trim().toUpperCase()}`;
  return `name:${normalizePersonName(nom)}|${normalizePersonName(prenom)}`;
}

function parseStudentAssignment(raw: unknown): StageStudentReferentAssignment | null {
  if (!raw || typeof raw !== "object") return null;
  const o = raw as Record<string, unknown>;
  const className = normalizeClassName(String(o.className ?? ""));
  const studentKey = String(o.studentKey ?? "").trim();
  const studentName = String(o.studentName ?? "").trim();
  const externalUserId = String(o.externalUserId ?? "").trim();
  const name = String(o.name ?? "").trim();
  const email = String(o.email ?? "").trim().toLowerCase();
  const eleveId = String(o.eleveId ?? "").trim() || undefined;
  const ine = String(o.ine ?? "").trim() || undefined;
  if (!className || !studentKey || !studentName || !externalUserId || !name || !email) return null;
  return { className, studentKey, studentName, externalUserId, name, email, eleveId, ine };
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
    studentAssignments: Array.isArray(hit.data.studentAssignments)
      ? hit.data.studentAssignments
          .map((row) => parseStudentAssignment(row))
          .filter((row): row is StageStudentReferentAssignment => row !== null)
      : [],
  };
}

export async function saveStageReferentsConfig(
  config: Omit<StageReferentsConfig, "studentAssignments"> & {
    studentAssignments?: StageStudentReferentAssignment[];
  },
): Promise<StageReferentsConfig> {
  const existing =
    config.studentAssignments === undefined
      ? await getStageReferentsConfig(config.schoolYear)
      : null;
  const studentAssignments = (config.studentAssignments ?? existing?.studentAssignments ?? [])
    .map((row) => parseStudentAssignment(row))
    .filter((row): row is StageStudentReferentAssignment => row !== null);

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
    studentAssignments,
  };
  await putJson(STAGE_S3.referentsConfig(next.schoolYear), next);
  return next;
}

export function findStudentReferentAssignments(
  config: StageReferentsConfig | null | undefined,
  className: string,
): StageStudentReferentAssignment[] {
  if (!config || !className.trim()) return [];
  const key = classKey(className);
  return config.studentAssignments.filter((a) => classKey(a.className) === key);
}

export function findStudentReferentAssignmentForStudent(
  config: StageReferentsConfig | null | undefined,
  params: {
    className: string;
    studentKey?: string;
    nom?: string;
    prenom?: string;
    ine?: string;
    eleveId?: string;
  },
): StageStudentReferentAssignment | null {
  const rows = findStudentReferentAssignments(config, params.className);
  if (rows.length === 0) return null;

  const key =
    params.studentKey?.trim() ||
    (params.nom && params.prenom
      ? stageRosterStudentKey(params.nom, params.prenom, params.ine)
      : "");
  const nameKey =
    params.nom && params.prenom ? stageRosterStudentKey(params.nom, params.prenom) : "";
  const eleveId = params.eleveId?.trim() || "";
  const ine = params.ine?.trim().toUpperCase() || "";

  return (
    rows.find((a) => key && a.studentKey === key) ??
    rows.find((a) => eleveId && a.eleveId && a.eleveId === eleveId) ??
    rows.find((a) => ine && a.ine?.trim().toUpperCase() === ine) ??
    rows.find((a) => nameKey && a.studentKey === nameKey) ??
    rows.find((a) => {
      if (!params.nom || !params.prenom) return false;
      const blob = normalizePersonName(a.studentName);
      return (
        blob.includes(normalizePersonName(params.nom)) &&
        blob.includes(normalizePersonName(params.prenom))
      );
    }) ??
    null
  );
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

async function resolvePrincipalForClass(
  className: string,
  schoolYear?: string,
): Promise<StageClassReferentAssignment | null> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStageReferentsConfig(year);
  const principals = findPrincipalAssignments(config, className);
  return principals[0] ?? null;
}

export async function resolvePrincipalSignerForClass(
  className: string,
  schoolYear?: string,
): Promise<StageClassReferentAssignment | null> {
  return resolvePrincipalForClass(className, schoolYear);
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
  const fromPool = config.assignments
    .filter((a) => a.externalUserId === id)
    .map((a) => a.className);
  const fromStudents = config.studentAssignments
    .filter((a) => a.externalUserId === id)
    .map((a) => a.className);
  return [...new Set([...fromPool, ...fromStudents])].sort((a, b) =>
    a.localeCompare(b, "fr", { sensitivity: "base" }),
  );
}

/**
 * Classes où l'utilisateur est professeur principal (voit toute la classe).
 * Compat : classe sans PP explicite → tout assigné historique compte comme PP.
 */
export async function listPrincipalClassesForUser(
  externalUserId: string,
  schoolYear?: string,
): Promise<string[]> {
  const id = externalUserId.trim();
  if (!id) return [];
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStageReferentsConfig(year);
  if (!config) return [];

  const byClass = new Map<string, StageClassReferentAssignment[]>();
  for (const a of config.assignments) {
    const key = classKey(a.className);
    const list = byClass.get(key) ?? [];
    list.push(a);
    byClass.set(key, list);
  }

  const out = new Set<string>();
  for (const list of byClass.values()) {
    const hasExplicitPrincipal = list.some((a) => a.role === "professeur_principal");
    for (const a of list) {
      if (a.externalUserId !== id) continue;
      if (a.role === "professeur_principal") {
        out.add(a.className);
        continue;
      }
      if (!hasExplicitPrincipal) {
        out.add(a.className);
      }
    }
  }

  return [...out].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
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

  const year = convention.schoolYear?.trim() || currentStageSchoolYear();
  const config = await getStageReferentsConfig(year);
  const studentHit = findStudentReferentAssignmentForStudent(config, {
    className: convention.student.className,
    nom: convention.student.lastName,
    prenom: convention.student.firstName,
  });
  if (!studentHit) return convention;

  return {
    ...convention,
    teacherReferent: {
      name: studentHit.name,
      email: studentHit.email,
      userId: studentHit.externalUserId,
    },
  };
}
