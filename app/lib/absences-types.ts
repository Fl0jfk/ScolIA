import type { AbsenceHoursTreatment } from "@/app/lib/absence-hours-treatment";
import type { AbsencePeriodType } from "@/app/lib/absence-period";

export type AbsenceScope = "professeur" | "ogec";
export type Etablissement = "École" | "Collège" | "Lycée";
export type AbsenceWorkflowStatus = "OUVERTE" | "JUSTIFICATIF_DEPOSE" | "CLOTUREE";
export type AbsenceDecision = "EN_ATTENTE" | "VALIDEE" | "REFUSEE";
export type AbsenceSource = "self" | "admin_manual" | "admin_pdf";

export type AbsenceRecord = {
  id: string;
  createdAt: string;
  updatedAt: string;
  source: AbsenceSource;
  displayName: string;
  calendarVisible: boolean;
  createdBy: {
    userId: string;
    name: string;
    email: string;
    roles: string[];
  };
  data: {
    scope: AbsenceScope;
    etablissement: Etablissement | null;
    periodType?: AbsencePeriodType | null;
    startDate: string;
    endDate: string;
    startTime?: string | null;
    endTime?: string | null;
    startAt: string;
    endAt: string;
    reason: string;
    details: string;
    sourceDocument?: string;
    documentKeys?: string[];
    confidence?: number;
  };
  workflowStatus: AbsenceWorkflowStatus;
  managerDecision: AbsenceDecision;
  closedAt?: string | null;
  justification?: {
    fileName: string;
    fileUrl: string;
    uploadedAt: string;
    uploadedBy: string;
  } | null;
  managerNote?: string;
  hoursTreatment?: AbsenceHoursTreatment | null;
  justificatifRelanceAt?: string | null;
  /** Motif masqué au calendrier (RGPD). */
  privacyReasonRedacted?: boolean;
  /** Date de suppression des pièces jointes sensibles. */
  privacyDocumentsPurgedAt?: string | null;
  history: Array<{
    at: string;
    by: string;
    action: string;
    note?: string;
  }>;
};

export const ABSENCES_INDEX_KEY = "absences/index.json";

function normRole(s: string) {
  return String(s || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, "");
}

function normRoleSpaced(s: string) {
  return String(s || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, " ")
    .trim();
}

export function hasRole(roles: string[], matcher: string) {
  const m = normRole(matcher);
  return roles.some((r) => normRole(r).includes(m));
}

function isTeacherRole(roles: string[]) {
  return roles.some((r) => normRole(r).includes("professeur"));
}

/** Personnel OGEC (hors enseignement) : admin, compta, éducation, direction. */
function isOgecStaffRole(roles: string[]) {
  const flags = getRoleFlags(roles);
  return (
    flags.isAdministratif ||
    flags.isCompta ||
    flags.isEducation ||
    flags.isDirectionEcole ||
    flags.isDirectionCollege ||
    flags.isDirectionLycee
  );
}

/** Détermine le scope d'une auto-déclaration (avec choix explicite si double casquette). */
export function resolveSelfDeclarationScope(roles: string[], requested?: unknown): AbsenceScope {
  const teacher = isTeacherRole(roles);
  const ogecStaff = isOgecStaffRole(roles);
  const req = requested === "ogec" || requested === "professeur" ? requested : null;

  if (teacher && !ogecStaff) return "professeur";
  if (ogecStaff && !teacher) return "ogec";
  if (teacher && ogecStaff) return req === "professeur" ? "professeur" : "ogec";
  return req === "professeur" ? "professeur" : "ogec";
}

export function canChooseDeclarationScope(roles: string[]) {
  return isTeacherRole(roles) && isOgecStaffRole(roles);
}

export function getRoleFlags(roles: string[]) {
  const spaced = roles.map((r) => normRoleSpaced(r));
  const hasToken = (token: string) => spaced.some((n) => n.includes(token));
  return {
    isDirectionEcole: hasToken("direction ecole") || hasRole(roles, "directionecole"),
    isDirectionCollege: hasToken("direction college") || hasRole(roles, "directioncollege"),
    isDirectionLycee: hasToken("direction lycee") || hasRole(roles, "directionlycee"),
    isCompta: hasToken("compta") || hasToken("comptabilite") || hasRole(roles, "comptabilite"),
    isAdministratif: hasToken("administratif"),
    isEducation: hasToken("education") || hasRole(roles, "cpe"),
  };
}

export function canViewCalendar(roles: string[]) {
  if (isTeacherRole(roles) && !getRoleFlags(roles).isAdministratif) return false;
  const flags = getRoleFlags(roles);
  if (flags.isCompta) return true;
  const normalized = roles.map((r) => normRoleSpaced(r));
  return normalized.some((r) =>
    ["administratif", "direction ecole", "direction college", "direction lycee", "education", "cpe"].some((allowed) =>
      r.includes(allowed),
    ),
  );
}

export function canAdminIngest(roles: string[]) {
  return canViewCalendar(roles);
}

/** Scope effectif (certains enregistrements legacy n'ont pas data.scope). */
export function resolveAbsenceScope(abs: AbsenceRecord): AbsenceScope {
  if (abs.data.scope === "ogec" || abs.data.scope === "professeur") return abs.data.scope;
  if (abs.data.etablissement) return "professeur";
  if (abs.source === "admin_manual" || abs.source === "admin_pdf") return "professeur";
  return "ogec";
}

/** Qui peut consulter les absences du personnel OGEC (hors les siennes). */
function canViewOgecAbsences(roles: string[]) {
  const flags = getRoleFlags(roles);
  return (
    flags.isAdministratif ||
    flags.isCompta ||
    flags.isDirectionEcole ||
    flags.isDirectionCollege ||
    flags.isDirectionLycee
  );
}

/** Visible sur le calendrier pour le viewer. */
export function isAbsenceVisibleOnCalendar(
  abs: AbsenceRecord,
  viewerUserId: string,
  roles: string[],
) {
  if (abs.managerDecision === "REFUSEE") return false;
  if (abs.calendarVisible) return true;
  if (abs.createdBy.userId === viewerUserId) return true;
  if (resolveAbsenceScope(abs) === "ogec") return canViewOgecAbsences(roles);
  return false;
}

/** Pièces jointes absences personnel OGEC : compta et direction uniquement (jamais administratif). */
function canViewOgecAbsenceAttachments(roles: string[]) {
  const flags = getRoleFlags(roles);
  if (flags.isCompta) return true;
  return flags.isDirectionEcole || flags.isDirectionCollege || flags.isDirectionLycee;
}

/** Pièces jointes absences professeurs : administratif et direction de l'établissement. */
/** Pièces jointes absences : jamais exposées dans l’UI (données sensibles). */
const ABSENCE_ATTACHMENTS_DISABLED = true;

function canViewProfAbsenceAttachments(abs: AbsenceRecord, roles: string[]) {
  if (ABSENCE_ATTACHMENTS_DISABLED) return false;
  const flags = getRoleFlags(roles);
  if (flags.isAdministratif) return true;
  const etab = abs.data.etablissement;
  if (etab === "École" && flags.isDirectionEcole) return true;
  if (etab === "Collège" && flags.isDirectionCollege) return true;
  if (etab === "Lycée" && flags.isDirectionLycee) return true;
  return false;
}

export function canViewAbsenceAttachment(abs: AbsenceRecord, viewerUserId: string, roles: string[]) {
  if (ABSENCE_ATTACHMENTS_DISABLED) return false;
  const scope = resolveAbsenceScope(abs);
  if (scope === "ogec") {
    if (abs.createdBy.userId === viewerUserId) return true;
    return canViewOgecAbsenceAttachments(roles);
  }
  return canViewProfAbsenceAttachments(abs, roles);
}

/** Ajout / suppression de pièces jointes (hors dépôt par le demandeur sur sa propre demande). */
export function canManageAbsenceAttachment(abs: AbsenceRecord, roles: string[]) {
  if (ABSENCE_ATTACHMENTS_DISABLED) return false;
  const scope = resolveAbsenceScope(abs);
  if (scope === "ogec") return canViewOgecAbsenceAttachments(roles);
  return canViewProfAbsenceAttachments(abs, roles);
}

function redactAbsenceAttachments(abs: AbsenceRecord): AbsenceRecord {
  return {
    ...abs,
    justification: null,
    data: {
      ...abs.data,
      documentKeys: undefined,
      sourceDocument: undefined,
    },
  };
}

export function filterAbsenceForViewer(abs: AbsenceRecord, viewerUserId: string, roles: string[]): AbsenceRecord {
  if (canViewAbsenceAttachment(abs, viewerUserId, roles)) return abs;
  return redactAbsenceAttachments(abs);
}

export function canViewAbsence(abs: AbsenceRecord, viewerUserId: string, roles: string[]) {
  if (abs.createdBy.userId === viewerUserId) return true;
  const flags = getRoleFlags(roles);
  const scope = resolveAbsenceScope(abs);
  if (scope === "ogec") {
    return canViewOgecAbsences(roles);
  }
  if (flags.isAdministratif || flags.isEducation) return true;
  if (abs.data.etablissement === "École") return flags.isDirectionEcole;
  if (abs.data.etablissement === "Collège") return flags.isDirectionCollege;
  if (abs.data.etablissement === "Lycée") return flags.isDirectionLycee;
  return false;
}

export function canManageAbsence(abs: AbsenceRecord, roles: string[]) {
  const flags = getRoleFlags(roles);
  const scope = resolveAbsenceScope(abs);
  if (scope === "ogec") return flags.isDirectionLycee;
  if (abs.data.etablissement === "École") return flags.isDirectionEcole;
  if (abs.data.etablissement === "Collège") return flags.isDirectionCollege;
  if (abs.data.etablissement === "Lycée") return flags.isDirectionLycee;
  return false;
}

/** File « À traiter » : décision en attente, hors saisie admin, pas sa propre déclaration. */
export function isAbsencePendingForManager(
  abs: AbsenceRecord,
  viewerUserId: string,
  roles: string[],
): boolean {
  if (abs.createdBy.userId === viewerUserId) return false;
  if (abs.managerDecision !== "EN_ATTENTE" || abs.workflowStatus === "CLOTUREE") return false;
  if (abs.source === "admin_manual" || abs.source === "admin_pdf") return false;
  return canManageAbsence(abs, roles);
}

export function parseLocalDateTime(dateStr: string, timeStr: string): Date | null {
  const ds = String(dateStr || "").trim();
  const ts = String(timeStr || "").trim();
  if (!ds || !ts) return null;
  const timeNorm = ts.length === 5 ? `${ts}:00` : ts;
  const [y, mo, d] = ds.split("-").map((v) => Number(v));
  const tp = timeNorm.split(":");
  const h = Number(tp[0]);
  const mi = Number(tp[1] ?? 0);
  const sec = Number(tp[2] ?? 0);
  if (![y, mo, d, h, mi, sec].every((n) => Number.isFinite(n))) return null;
  const dt = new Date(y, mo - 1, d, h, mi, sec, 0);
  if (Number.isNaN(dt.getTime())) return null;
  return dt;
}

export function computeStartEndAt(input: {
  periodType?: AbsencePeriodType | null;
  startDate: string;
  endDate: string;
  startTime?: string | null;
  endTime?: string | null;
}): { startAt: string; endAt: string } {
  if (input.periodType === "single_day" && input.startTime && input.endTime) {
    const start = parseLocalDateTime(input.startDate, input.startTime);
    const end = parseLocalDateTime(input.endDate, input.endTime);
    if (start && end) return { startAt: start.toISOString(), endAt: end.toISOString() };
  }
  const start = parseLocalDateTime(input.startDate, "08:00");
  const end = parseLocalDateTime(input.endDate, "18:00");
  return {
    startAt: (start || new Date(`${input.startDate}T08:00:00`)).toISOString(),
    endAt: (end || new Date(`${input.endDate}T18:00:00`)).toISOString(),
  };
}

/** Normalise un enregistrement (legacy self ou anciennes convocations migrées). */
export function normalizeAbsenceRecord(raw: AbsenceRecord): AbsenceRecord {
  const data = raw.data ?? {
    scope: "professeur" as const,
    etablissement: null,
    startDate: "",
    endDate: "",
    startAt: "",
    endAt: "",
    reason: "",
    details: "",
  };

  const source: AbsenceSource =
    raw.source === "admin_manual" || raw.source === "admin_pdf" || raw.source === "self"
      ? raw.source
      : raw.managerDecision === "VALIDEE" && raw.workflowStatus === "CLOTUREE" && !raw.createdBy?.userId
        ? "admin_pdf"
        : "self";

  const displayName =
    raw.displayName?.trim() ||
    (data as { teacherName?: string }).teacherName?.trim() ||
    raw.createdBy?.name ||
    "Inconnu";

  const reason =
    data.reason?.trim() ||
    (data as { examType?: string }).examType?.trim() ||
    "Absence";

  const { startAt, endAt } =
    data.startAt && data.endAt
      ? { startAt: data.startAt, endAt: data.endAt }
      : computeStartEndAt({
          periodType: data.periodType,
          startDate: data.startDate,
          endDate: data.endDate,
          startTime: data.startTime,
          endTime: data.endTime,
        });

  const calendarVisible =
    typeof raw.calendarVisible === "boolean"
      ? raw.calendarVisible
      : source === "admin_manual" || source === "admin_pdf"
        ? true
        : raw.managerDecision === "VALIDEE";

  const scope: AbsenceScope =
    data.scope === "ogec" || data.scope === "professeur"
      ? data.scope
      : data.etablissement
        ? "professeur"
        : source === "admin_manual" || source === "admin_pdf"
          ? "professeur"
          : "ogec";

  return {
    ...raw,
    source,
    displayName,
    calendarVisible,
    data: {
      ...data,
      scope,
      etablissement: scope === "ogec" ? null : data.etablissement ?? null,
      reason,
      details: data.details ?? "",
      startAt,
      endAt,
      documentKeys: data.documentKeys,
    },
  };
}

export function buildAdminAbsenceRecord(params: {
  source: "admin_manual" | "admin_pdf";
  displayName: string;
  scope?: AbsenceScope;
  etablissement: Etablissement | null;
  reason: string;
  startAt: string;
  endAt: string;
  documentKeys?: string[];
  sourceDocument?: string;
  confidence?: number;
  createdBy: AbsenceRecord["createdBy"];
}): AbsenceRecord {
  const now = new Date().toISOString();
  const id = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  const documentKeys = params.documentKeys?.filter(Boolean) ?? [];
  const scope = params.scope ?? "professeur";
  return {
    id,
    createdAt: now,
    updatedAt: now,
    source: params.source,
    displayName: params.displayName,
    calendarVisible: true,
    createdBy: params.createdBy,
    data: {
      scope,
      etablissement: scope === "ogec" ? null : params.etablissement,
      periodType: params.startAt.slice(0, 10) === params.endAt.slice(0, 10) ? "single_day" : "multi_day",
      startDate: params.startAt.slice(0, 10),
      endDate: params.endAt.slice(0, 10),
      startTime: null,
      endTime: null,
      startAt: params.startAt,
      endAt: params.endAt,
      reason: params.reason,
      details: "",
      sourceDocument: params.sourceDocument,
      documentKeys,
      confidence: params.confidence ?? 1,
    },
    workflowStatus: "CLOTUREE",
    managerDecision: "VALIDEE",
    closedAt: now,
    justification: null,
    justificatifRelanceAt: null,
    history: [
      {
        at: now,
        by: params.createdBy.name,
        action: params.source === "admin_pdf" ? "IMPORT_PDF" : "SAISIE_MANUELLE",
      },
    ],
  };
}

export function normalizeEtablissement(value: string): Etablissement {
  const n = normRoleSpaced(value);
  if (n.includes("ecole")) return "École";
  if (n.includes("lycee")) return "Lycée";
  return "Collège";
}
