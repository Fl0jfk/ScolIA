import type { EleveConfig } from "@/app/lib/eleves-config";
import { listElevesFromDb, resolveCurrentEtablissementId } from "@/app/lib/ent-core-db";
import {
  getStagePeriodsForClass,
  isClassEnabledInStagePeriods,
  listStageEnabledClassNames,
  type StageClassPeriod,
} from "@/app/lib/stage-periods-config";
import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";
import { classKey, stageRosterStudentKey } from "@/app/lib/stage-referents-config";
import { getConventionsIndex } from "@/app/lib/stage-storage";
import { loadStageConventionsByIds } from "@/app/lib/stage-convention-load";
import { buildSignatureSummary, type StageSignatureSummary } from "@/app/lib/stage-signature-summary";
import {
  currentStageSchoolYear,
  STAGE_CONVENTION_STATUS_LABELS,
  type StageConvention,
  type StageConventionIndexEntry,
  type StageConventionStatus,
} from "@/app/lib/stage-types";
import { valkeyCached } from "@/app/lib/valkey";
import { VALKEY_TTL, valkeyKeyStagesClassRoster } from "@/app/lib/valkey-keys";

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
  teacherReferentName?: string;
  teacherReferentEmail?: string;
  teacherReferentUserId?: string;
};

export type StageRosterStudent = {
  key: string;
  nom: string;
  prenom: string;
  ine?: string;
  /** Id Postgres dossier élève (photos / lien). */
  eleveId?: string;
  photoKey?: string;
  /** URL photo signée (renseignée par l’API roster). */
  photoUrl?: string | null;
  folderName?: string;
  rosterStatus: StageRosterStudentStatus;
  conventions: StageRosterConvention[];
  assignedReferentName?: string;
  assignedReferentEmail?: string;
  assignedReferentUserId?: string;
};

export type StageClassRoster = {
  className: string;
  schoolYear: string;
  /** True si des périodes officielles sont configurées (stage attendu pour la classe). */
  expectsMandatoryStage: boolean;
  officialPeriods: StageClassPeriod[];
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
  return schoolClassesMatch(resolved, className);
}

async function loadElevesForClass(className: string): Promise<EleveConfig[]> {
  const etabId = await resolveCurrentEtablissementId().catch(() => null);
  if (!etabId) return [];
  const trimmed = className.trim();
  if (!trimmed) return [];
  const rows = await listElevesFromDb(etabId, { status: "inscrit", classe: trimmed });
  return rows.filter((e) => eleveMatchesClass(e, className));
}

function isTerminalStatus(status: StageConventionStatus): boolean {
  return status === "cancelled" || status === "archived";
}

function rosterStatusFromConventions(conventions: StageRosterConvention[]): StageRosterStudentStatus {
  const active = conventions.filter(
    (c) => !isTerminalStatus(c.status) && c.status !== "draft",
  );
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
    teacherReferentName: c.teacherReferent.name?.trim() || undefined,
    teacherReferentEmail: c.teacherReferent.email?.trim() || undefined,
    teacherReferentUserId: c.teacherReferent.userId?.trim() || undefined,
  };
}

function studentKey(nom: string, prenom: string, ine?: string): string {
  return stageRosterStudentKey(nom, prenom, ine);
}

function isRosterVisibleConvention(c: StageConvention, schoolYear: string): boolean {
  // Brouillon élève : visible uniquement côté élève, pas dans le suivi admin / classe.
  if (c.status === "archived" || c.status === "cancelled" || c.status === "draft") return false;
  if (c.schoolYear === schoolYear) return true;
  // Même année calendaire de stage mais schoolYear mal renseigné / N-1 encore actif :
  // Absences repas les listait déjà ; le suivi classe doit les retrouver.
  return (
    c.status === "signed" ||
    c.status === "signatures_pending" ||
    c.status === "convention_ready"
  );
}

/**
 * Classes disponibles dans le suivi : config stages activée + classes
 * ayant déjà un dossier (stages volontaires hors config, ex. terminale).
 */
function normalizeSearchBlob(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

export type StageGlobalSearchHit = {
  conventionId: string;
  className: string;
  studentFirstName: string;
  studentLastName: string;
  companyName: string;
  status: StageConventionStatus;
  statusLabel: string;
  stageLabel?: string;
};

/**
 * Recherche globale (toutes classes ayant déjà un dossier / suivies) :
 * nom élève, entreprise, classe, libellé de période.
 */
function isRosterVisibleIndexEntry(
  entry: { status: StageConventionStatus; schoolYear: string },
  schoolYear: string,
): boolean {
  if (entry.status === "archived" || entry.status === "cancelled" || entry.status === "draft") {
    return false;
  }
  if (entry.schoolYear === schoolYear) return true;
  return (
    entry.status === "signed" ||
    entry.status === "signatures_pending" ||
    entry.status === "convention_ready"
  );
}

function splitStudentName(studentName: string): { firstName: string; lastName: string } {
  const parts = studentName.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return { firstName: "", lastName: "" };
  if (parts.length === 1) return { firstName: parts[0]!, lastName: "" };
  // Index : `${firstName} ${lastName}` — le 1er token = prénom, le reste = nom.
  return {
    firstName: parts[0]!,
    lastName: parts.slice(1).join(" "),
  };
}

export async function searchStageConventionsGlobal(
  query: string,
  opts?: {
    schoolYear?: string;
    /** Si fourni, limite aux classes autorisées (référents). */
    allowedClasses?: string[] | null;
  },
): Promise<StageGlobalSearchHit[]> {
  const q = normalizeSearchBlob(query);
  if (q.length < 2) return [];

  const year = opts?.schoolYear?.trim() || currentStageSchoolYear();
  const allowed = opts?.allowedClasses?.length
    ? opts.allowedClasses.map((c) => c.trim()).filter(Boolean)
    : null;

  const index = await getConventionsIndex();
  const hits: StageGlobalSearchHit[] = [];

  for (const entry of index) {
    if (!isRosterVisibleIndexEntry(entry, year)) continue;
    if (allowed) {
      const className = String(entry.className ?? "").trim();
      const ok = allowed.some((a) => schoolClassesMatch(className, a));
      if (!ok) continue;
    }

    const indexBlob = normalizeSearchBlob(
      [entry.studentName, entry.companyName, entry.className, entry.stageLabel]
        .filter(Boolean)
        .join(" "),
    );
    if (!indexBlob.includes(q)) continue;

    const { firstName, lastName } = splitStudentName(entry.studentName);
    hits.push({
      conventionId: entry.id,
      className: String(entry.className ?? "").trim(),
      studentFirstName: firstName,
      studentLastName: lastName,
      companyName: entry.companyName || "—",
      status: entry.status,
      statusLabel: STAGE_CONVENTION_STATUS_LABELS[entry.status] || entry.status,
      stageLabel: entry.stageLabel?.trim() || undefined,
    });
    if (hits.length >= 40) break;
  }

  hits.sort((a, b) => {
    const ln = a.studentLastName.localeCompare(b.studentLastName, "fr", { sensitivity: "base" });
    if (ln !== 0) return ln;
    const fn = a.studentFirstName.localeCompare(b.studentFirstName, "fr", { sensitivity: "base" });
    if (fn !== 0) return fn;
    return a.className.localeCompare(b.className, "fr", { sensitivity: "base" });
  });

  return hits;
}

export async function listStageRosterClassNames(
  schoolYear?: string,
  index?: StageConventionIndexEntry[],
): Promise<string[]> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const [enabled, resolvedIndex] = await Promise.all([
    listStageEnabledClassNames(year),
    index ? Promise.resolve(index) : getConventionsIndex(),
  ]);
  const fromConventions = resolvedIndex
    .filter((e) => isRosterVisibleIndexEntry(e, year))
    .map((e) => String(e.className ?? "").trim())
    .filter(Boolean);
  return [...new Set([...enabled, ...fromConventions])].sort((a, b) =>
    a.localeCompare(b, "fr", { sensitivity: "base" }),
  );
}

async function buildStageClassRosterUncached(
  className: string,
  year: string,
  opts?: { index?: StageConventionIndexEntry[] },
): Promise<StageClassRoster> {
  const indexPromise = opts?.index
    ? Promise.resolve(opts.index)
    : getConventionsIndex();
  const [eleves, index, officialPeriods, classEnabledInConfig] = await Promise.all([
    loadElevesForClass(className),
    indexPromise,
    getStagePeriodsForClass(className, year),
    isClassEnabledInStagePeriods(className, year),
  ]);
  const expectsMandatoryStage = officialPeriods.length > 0;
  /** Classe ouverte aux stages dans les réglages → toujours lister tout le registre (lycée sans période, etc.). */
  const listFullClassRoster = expectsMandatoryStage || classEnabledInConfig;
  const classEleves = eleves.filter((e) => eleveMatchesClass(e, className));

  const candidateIds = index
    .filter((e) => {
      if (!isRosterVisibleIndexEntry(e, year)) return false;
      return schoolClassesMatch(String(e.className ?? ""), className);
    })
    .map((e) => e.id);

  const conventions = (await loadStageConventionsByIds(candidateIds)).filter((c) =>
    isRosterVisibleConvention(c, year),
  );

  const studentMap = new Map<string, StageRosterStudent>();

  for (const eleve of classEleves) {
    const key = studentKey(eleve.nom, eleve.prenom, eleve.ine);
    studentMap.set(key, {
      key,
      nom: eleve.nom,
      prenom: eleve.prenom,
      ine: eleve.ine || undefined,
      eleveId: eleve.id?.trim() || undefined,
      photoKey: eleve.photoKey?.trim() || undefined,
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

    if (!row.eleveId) {
      const matchedEleve = classEleves.find((e) => namesMatch(e, convention.student));
      if (matchedEleve?.id?.trim()) {
        row.eleveId = matchedEleve.id.trim();
        row.photoKey = matchedEleve.photoKey?.trim() || row.photoKey;
        row.ine = row.ine || matchedEleve.ine || undefined;
      }
    }

    row.conventions.push(toRosterConvention(convention));
    studentMap.set(key, row);
  }

  const studentsRaw = [...studentMap.values()]
    .map((s) => {
      s.conventions.sort((a, b) => b.periodStart.localeCompare(a.periodStart));
      return { ...s, rosterStatus: rosterStatusFromConventions(s.conventions) };
    })
    .sort((a, b) => {
      const ln = a.nom.localeCompare(b.nom, "fr", { sensitivity: "base" });
      if (ln !== 0) return ln;
      return a.prenom.localeCompare(b.prenom, "fr", { sensitivity: "base" });
    });

  /**
   * Classe hors config stages (arrivée via un dossier isolé) : ne lister que les
   * élèves ayant déjà une convention. Dès qu’elle est activée dans Réglages
   * (même sans période officielle, ex. lycée), on affiche toute la classe.
   */
  const students = listFullClassRoster
    ? studentsRaw
    : studentsRaw.filter((s) => s.conventions.length > 0);

  const summary = {
    total: students.length,
    sansStage: students.filter((s) => s.rosterStatus === "sans_stage").length,
    enCours: students.filter((s) => s.rosterStatus === "en_cours").length,
    valide: students.filter((s) => s.rosterStatus === "valide").length,
    plusieurs: students.filter((s) => s.rosterStatus === "plusieurs").length,
  };

  const rosterSource = classEleves.length > 0 ? "eleves_and_conventions" : "conventions_only";
  const notes: string[] = [];
  if (classEleves.length === 0) {
    notes.push(
      "Liste élèves vide pour cette classe — seuls les dossiers de stage déjà ouverts sont affichés. Renseignez le champ « classe » dans le registre élèves pour un suivi complet.",
    );
  }
  if (classEnabledInConfig && !expectsMandatoryStage) {
    notes.push(
      "Aucune période officielle pour cette classe : le suivi liste toute la classe (stages volontaires possibles). Ajoutez des périodes dans Stages → Réglages si besoin.",
    );
  } else if (!listFullClassRoster) {
    notes.push(
      "Cette classe n’est pas activée dans les réglages stages : seuls les élèves ayant déjà un dossier apparaissent ici.",
    );
  }

  return {
    className,
    schoolYear: year,
    expectsMandatoryStage,
    officialPeriods,
    summary,
    students,
    rosterSource,
    ...(notes.length ? { note: notes.join(" ") } : {}),
  };
}

export async function buildStageClassRoster(
  className: string,
  schoolYear?: string,
  opts?: { index?: StageConventionIndexEntry[] },
): Promise<StageClassRoster> {
  const year = schoolYear?.trim() || currentStageSchoolYear();
  const etabId = await resolveCurrentEtablissementId().catch(() => null);
  if (!etabId) return buildStageClassRosterUncached(className, year, opts);

  return valkeyCached({
    key: valkeyKeyStagesClassRoster(etabId, year, classKey(className)),
    ttlSeconds: VALKEY_TTL.stagesClassRoster,
    loader: () => buildStageClassRosterUncached(className, year, opts),
  });
}
