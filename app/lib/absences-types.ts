import type { AbsenceHoursTreatment } from "@/app/lib/absence-hours-treatment";
import type { AbsencePeriodType } from "@/app/lib/absence-period";
import type { Establishment } from "@/app/lib/app-config-schemas";
import {
  directionRolesMatchEstablishmentRef,
  isAnyDirectionRole,
} from "@/app/lib/establishment-catalog";
import { parseParisDateTime, parisDateKey } from "@/app/lib/paris-time";

export type AbsenceScope = "professeur" | "ogec";
export type Etablissement = string;
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
  /** Si renseigné : la demande a été saisie par un administratif pour le compte de createdBy. */
  submittedBy?: {
    userId: string;
    name: string;
    email: string;
    roles: string[];
  } | null;
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
    flags.isDirection
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

export type DirectionAuthCtx = {
  establishments?: Establishment[];
  userId?: string | null;
};

export function getRoleFlags(roles: string[]) {
  const spaced = roles.map((r) => normRoleSpaced(r));
  const hasToken = (token: string) => spaced.some((n) => n.includes(token));
  const isDirection = isAnyDirectionRole(roles);
  return {
    isDirectionEcole: hasToken("direction ecole") || hasRole(roles, "directionecole"),
    isDirectionCollege: hasToken("direction college") || hasRole(roles, "directioncollege"),
    isDirectionLycee: hasToken("direction lycee") || hasRole(roles, "directionlycee"),
    isDirection,
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
    ["administratif", "direction ecole", "direction college", "direction lycee", "direction", "education", "cpe"].some((allowed) =>
      r.includes(allowed),
    ),
  );
}

export function canAdminIngest(roles: string[]) {
  return canViewCalendar(roles);
}

/** Administratif, comptabilité et direction uniquement — pas CPE, profs, éducation, etc. */
export function canDeclareAbsenceOnBehalf(roles: string[]) {
  const flags = getRoleFlags(roles);
  return flags.isAdministratif || flags.isCompta || flags.isDirection;
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
  return flags.isAdministratif || flags.isCompta || flags.isDirection;
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
  return flags.isDirection;
}

/** Pièces jointes absences professeurs : administratif et direction de l'établissement. */
function canViewProfAbsenceAttachments(
  abs: AbsenceRecord,
  roles: string[],
  ctx?: DirectionAuthCtx,
) {
  const flags = getRoleFlags(roles);
  if (flags.isAdministratif) return true;
  return directionRolesMatchEstablishmentRef(
    roles,
    abs.data.etablissement,
    ctx?.establishments,
    ctx?.userId,
  );
}

export function canViewAbsenceAttachment(
  abs: AbsenceRecord,
  viewerUserId: string,
  roles: string[],
  ctx?: DirectionAuthCtx,
) {
  if (abs.createdBy.userId === viewerUserId) return true;
  const scope = resolveAbsenceScope(abs);
  if (scope === "ogec") return canViewOgecAbsenceAttachments(roles);
  return canViewProfAbsenceAttachments(abs, roles, ctx);
}

/** Ajout / suppression de pièces jointes (hors dépôt par le demandeur sur sa propre demande). */
export function canManageAbsenceAttachment(
  abs: AbsenceRecord,
  roles: string[],
  ctx?: DirectionAuthCtx,
) {
  const scope = resolveAbsenceScope(abs);
  if (scope === "ogec") return canViewOgecAbsenceAttachments(roles);
  return canViewProfAbsenceAttachments(abs, roles, ctx);
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

export function filterAbsenceForViewer(
  abs: AbsenceRecord,
  viewerUserId: string,
  roles: string[],
  ctx?: DirectionAuthCtx,
): AbsenceRecord {
  if (canViewAbsenceAttachment(abs, viewerUserId, roles, ctx)) return abs;
  return redactAbsenceAttachments(abs);
}

/** Personnel « éducation / surveillance » (vie scolaire) — hors CPE. */
export function isEducationSurveillanceStaff(roles: string[] | null | undefined): boolean {
  if (!Array.isArray(roles) || roles.length === 0) return false;
  return roles.some((r) => {
    const n = normRole(r);
    return n === "education" || n.includes("surveillant");
  });
}

export function canViewAbsence(
  abs: AbsenceRecord,
  viewerUserId: string,
  roles: string[],
  ctx?: DirectionAuthCtx,
) {
  if (abs.createdBy.userId === viewerUserId) return true;
  const flags = getRoleFlags(roles);
  const scope = resolveAbsenceScope(abs);
  if (scope === "ogec") {
    return canViewOgecAbsences(roles);
  }
  if (flags.isAdministratif || flags.isEducation) return true;
  return directionRolesMatchEstablishmentRef(
    roles,
    abs.data.etablissement,
    ctx?.establishments,
    viewerUserId,
  );
}

export function canManageAbsence(abs: AbsenceRecord, roles: string[], ctx?: DirectionAuthCtx) {
  const flags = getRoleFlags(roles);
  const scope = resolveAbsenceScope(abs);
  if (scope === "ogec") return flags.isDirection;
  return directionRolesMatchEstablishmentRef(
    roles,
    abs.data.etablissement,
    ctx?.establishments,
    ctx?.userId,
  );
}

/** File « À traiter » : décision en attente, hors saisie admin, pas sa propre déclaration. */
export function isAbsencePendingForManager(
  abs: AbsenceRecord,
  viewerUserId: string,
  roles: string[],
  ctx?: DirectionAuthCtx,
): boolean {
  if (abs.createdBy.userId === viewerUserId) return false;
  if (abs.managerDecision !== "EN_ATTENTE" || abs.workflowStatus === "CLOTUREE") return false;
  if (abs.source === "admin_manual" || abs.source === "admin_pdf") return false;
  return canManageAbsence(abs, roles, { ...ctx, userId: viewerUserId });
}

/**
 * Heure murale établissement (Europe/Paris) → Date UTC.
 * Ne pas utiliser `new Date(y, m, d, h, …)` : sur un serveur UTC ça stocke
 * l’heure saisie comme si elle était déjà en UTC (+2 h à l’affichage Paris en été).
 */
export function parseLocalDateTime(dateStr: string, timeStr: string): Date | null {
  return parseParisDateTime(dateStr, timeStr);
}

export function computeStartEndAt(input: {
  periodType?: AbsencePeriodType | null;
  startDate: string;
  endDate: string;
  startTime?: string | null;
  endTime?: string | null;
}): { startAt: string; endAt: string } {
  if (input.periodType === "single_day" && input.startTime && input.endTime) {
    const start = parseParisDateTime(input.startDate, input.startTime);
    const end = parseParisDateTime(input.endDate, input.endTime);
    if (start && end) return { startAt: start.toISOString(), endAt: end.toISOString() };
  }
  const start = parseParisDateTime(input.startDate, "08:00");
  const end = parseParisDateTime(input.endDate, "18:00");
  const startAt = start ?? new Date();
  const endAt = end ?? startAt;
  return {
    startAt: startAt.toISOString(),
    endAt: endAt.toISOString(),
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
      periodType: parisDateKey(params.startAt) === parisDateKey(params.endAt) ? "single_day" : "multi_day",
      startDate: parisDateKey(params.startAt),
      endDate: parisDateKey(params.endAt),
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
  const raw = String(value || "").trim();
  if (!raw) return raw;
  const n = normRoleSpaced(raw);
  if (n.includes("ecole") && !n.includes("college") && !n.includes("lycee")) return "École";
  if (n.includes("college")) return "Collège";
  if (n.includes("lycee")) return "Lycée";
  return raw;
}
