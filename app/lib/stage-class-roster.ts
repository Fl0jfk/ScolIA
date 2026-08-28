import type { EleveConfig } from "@/app/lib/eleves-config";
import { loadElevesRegistry } from "@/app/lib/eleves-registry";
import { classKey } from "@/app/lib/stage-referents-config";
import { getConventionsIndex, getStageConvention } from "@/app/lib/stage-storage";
import { buildSignatureSummary, type StageSignatureSummary } from "@/app/lib/stage-signature-summary";
import {
  currentStageSchoolYear,
  STAGE_CONVENTION_STATUS_LABELS,
  type StageConvention,
  type StageConventionStatus,
} from "@/app/lib/stage-types";

export type StageRosterStudentStatus = "sans_stage" | "en_cours" | "valide" | "plusieurs";

type StageRosterConvention = {
  id: string;
  status: StageConventionStatus;
  statusLabel: string;
  stageLabel?: string;
  companyName: string;
  periodStart: string;
  periodEnd: string;
  internshipKind: string;
  oneDriveFiled: boolean;
  canFileOneDrive: boolean;
  signatureSummary: StageSignatureSummary;
};

export type StageRosterStudent = {
  key: string;
  nom: string;
  prenom: string;
  ine?: string;
  folderName?: string;
  rosterStatus: StageRosterStudentStatus;
  conventions: StageRosterConvention[];
};

export type StageClassRoster = {
  className: string;
  schoolYear: string;
  summary: {
    total: number;
    sansStage: number;
    enCours: number;
    valide: number;
    plusieurs: number;
  };
  students: StageRosterStudent[];
  rosterSource: "eleves_and_conventions" | "conventions_only";
  note?: string;
};

function normalizeName(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[-\s]+/g, " ")
    .trim();
}

function namesMatch(
  a: { nom: string; prenom: string },
  b: { lastName: string; firstName: string },
): boolean {
  const an = normalizeName(a.nom);
  const ap = normalizeName(a.prenom);
  const bn = normalizeName(b.lastName);
  const bp = normalizeName(b.firstName);
  return an === bn && ap === bp;
}

/** Classe explicite (champ `classe` de la liste élèves). */
function resolveEleveClassName(eleve: EleveConfig): string | null {
  const explicit = String(eleve.classe ?? "").trim();
  if (explicit) return explicit;

  const parts = eleve.folderName
    .split(/[—–\-]/)
    .map((p) => p.trim())
    .filter(Boolean);
  if (parts.length < 2) return null;
  const last = parts[parts.length - 1]!;
  if (parts.length >= 3) return last;
  if (/^(\d+e|\d+\s*e|2nde|seconde|1re|1ère|premiere|terminale|tle|cap|bts)/i.test(last)) {
    return last;
  }
  return null;
}

function eleveMatchesClass(eleve: EleveConfig, className: string): boolean {
  const resolved = resolveEleveClassName(eleve);
  if (!resolved) return false;
  return classKey(resolved) === classKey(className);
}

async function loadEleves(): Promise<EleveConfig[]> {
  return loadElevesRegistry();
}

function isTerminalStatus(status: StageConventionStatus): boolean {
  return status === "cancelled" || status === "archived";
}

function rosterStatusFromConventions(conventions: StageRosterConvention[]): StageRosterStudentStatus {
  const active = conventions.filter((c) => !isTerminalStatus(c.status));
  if (active.length === 0) return "sans_stage";
  const signed = active.filter((c) => c.status === "signed");
  const inProgress = active.filter((c) => c.status !== "signed");
  if (active.length > 1) return "plusieurs";
  if (signed.length === 1 && inProgress.length === 0) return "valide";
  return "en_cours";
}

function toRosterConvention(c: StageConvention): StageRosterConvention {
  return {
    id: c.id,
    status: c.status,
    statusLabel: STAGE_CONVENTION_STATUS_LABELS[c.status] || c.status,
    stageLabel: c.stageLabel,
    companyName: c.company.name,
    periodStart: c.schedule.periodStart,
    periodEnd: c.schedule.periodEnd,
    internshipKind: c.internshipKind,
    oneDriveFiled: Boolean(c.oneDriveFiling?.filedAt),
    canFileOneDrive: c.status === "signed" && !c.oneDriveFiling?.filedAt,
    signatureSummary: buildSignatureSummary(c),
  };
}

function studentKey(nom: string, prenom: string, ine?: string): string {
  if (ine?.trim()) return `ine:${ine.trim().toUpperCase()}`;
  return `name:${normalizeName(nom)}|${normalizeName(prenom)}`;
}

export async function buildStageClassRoster(
  className: string,
  schoolYear?: string,
): Promise<StageClassRoster> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const classK = classKey(className);

  const [eleves, index] = await Promise.all([loadEleves(), getConventionsIndex()]);
  const classEleves = eleves.filter((e) => eleveMatchesClass(e, className));

  const conventions = (
    await Promise.all(
      index
        .filter((e) => e.schoolYear === year && classKey(e.className) === classK)
        .map((e) => getStageConvention(e.id)),
    )
  ).filter((c): c is StageConvention => Boolean(c));

  const studentMap = new Map<string, StageRosterStudent>();

  for (const eleve of classEleves) {
    const key = studentKey(eleve.nom, eleve.prenom, eleve.ine);
    studentMap.set(key, {
      key,
      nom: eleve.nom,
      prenom: eleve.prenom,
      ine: eleve.ine || undefined,
      folderName: eleve.folderName,
      rosterStatus: "sans_stage",
      conventions: [],
    });
  }

  for (const convention of conventions) {
    const matchedKey = [...studentMap.entries()].find(([, s]) =>
      namesMatch(s, convention.student),
    )?.[0];

    const key =
      matchedKey ??
      studentKey(convention.student.lastName, convention.student.firstName);

    const existing = studentMap.get(key);
    const row: StageRosterStudent = existing ?? {
      key,
      nom: convention.student.lastName,
      prenom: convention.student.firstName,
      rosterStatus: "sans_stage",
      conventions: [],
    };

    row.conventions.push(toRosterConvention(convention));
    studentMap.set(key, row);
  }

  const students = [...studentMap.values()]
    .map((s) => {
      s.conventions.sort((a, b) => b.periodStart.localeCompare(a.periodStart));
      return { ...s, rosterStatus: rosterStatusFromConventions(s.conventions) };
    })
    .sort((a, b) => {
      const ln = a.nom.localeCompare(b.nom, "fr", { sensitivity: "base" });
      if (ln !== 0) return ln;
      return a.prenom.localeCompare(b.prenom, "fr", { sensitivity: "base" });
    });

  const summary = {
    total: students.length,
    sansStage: students.filter((s) => s.rosterStatus === "sans_stage").length,
    enCours: students.filter((s) => s.rosterStatus === "en_cours").length,
    valide: students.filter((s) => s.rosterStatus === "valide").length,
    plusieurs: students.filter((s) => s.rosterStatus === "plusieurs").length,
  };

  const rosterSource = classEleves.length > 0 ? "eleves_and_conventions" : "conventions_only";
  const note =
    classEleves.length === 0
      ? "Liste élèves vide pour cette classe — seuls les dossiers de stage déjà ouverts sont affichés. Renseignez le champ « classe » dans eleves.json pour un suivi complet."
      : undefined;

  return {
    className,
    schoolYear: year,
    summary,
    students,
    rosterSource,
    ...(note ? { note } : {}),
  };
}
