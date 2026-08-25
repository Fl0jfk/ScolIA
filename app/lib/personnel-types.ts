import { hasRole } from "@/app/lib/absences-types";
import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import { defaultPersonnelProfile, normalizePersonnelProfile, type PersonnelProfile } from "@/app/lib/personnel-profile";

/** Phase pilote : accès ouvert à tous les utilisateurs connectés. À restreindre plus tard. */
const PERSONNEL_OPEN_ACCESS = true;

export type { PersonnelProfile };

export type PersonnelCategory =
  | "administratif"
  | "maintenance"
  | "surveillant"
  | "cpe"
  | "comptabilite";

export type DocumentVisibility = "personnel" | "establishment" | "restricted";

export type PersonnelDocCategory =
  | "contrat"
  | "formation"
  | "habilitation"
  | "medecine"
  | "entretien"
  | "onboarding"
  | "autre";

export type PersonnelDocument = {
  id: string;
  name: string;
  fileUrl: string;
  s3Key?: string;
  category: PersonnelDocCategory;
  visibility: DocumentVisibility;
  uploadedAt: string;
  uploadedBy: string;
  expiresAt?: string | null;
};

type FormationStatus = "demandee" | "planifiee" | "realisee" | "annulee";

export type PersonnelFormation = {
  id: string;
  title: string;
  status: FormationStatus;
  plannedDate?: string | null;
  completedDate?: string | null;
  reminderAt?: string | null;
  documentId?: string | null;
  notes?: string;
};

export type PersonnelHabilitation = {
  id: string;
  label: string;
  obtainedAt?: string | null;
  expiresAt: string;
  documentId?: string | null;
  notes?: string;
};

export type PersonnelMedecineVisit = {
  id: string;
  visitedAt: string;
  visitType?: string;
  documentId?: string | null;
  notes?: string;
  createdAt: string;
};

export type PersonnelMedecineTravail = {
  visits?: PersonnelMedecineVisit[];
  lastVisitAt?: string | null;
  nextVisitAt?: string | null;
  visitType?: string;
  documentId?: string | null;
  notes?: string;
};

export type EntretienStatus = "a_planifier" | "planifie" | "realise";

export type PersonnelEntretien = {
  id: string;
  scheduledAt?: string | null;
  completedAt?: string | null;
  reminderAt?: string | null;
  nextDueAt?: string | null;
  documentId?: string | null;
  notes?: string;
  status: EntretienStatus;
};

export type OnboardingStatus = "brouillon" | "en_cours" | "signatures" | "termine" | "annule";

export type OnboardingSignature = {
  id: string;
  role: "employe" | "directrice" | "compta" | "president_ogec";
  label: string;
  status: "en_attente" | "signe";
  signedAt?: string | null;
  signedBy?: string | null;
  signEmail?: string | null;
  signToken?: string | null;
  signSentAt?: string | null;
  signResponseIp?: string | null;
};

export type OffboardingStatus = "en_cours" | "termine";

export type PersonnelOffboarding = {
  id: string;
  status: OffboardingStatus;
  endDate: string;
  checklist: Array<{ id: string; label: string; done: boolean }>;
  signatures: OnboardingSignature[];
  notes?: string;
};

export type PersonnelLeaveType = "conge_paye" | "rtt" | "sans_solde" | "autre";
export type PersonnelLeaveStatus = "en_attente" | "validee" | "refusee" | "annulee";

export type PersonnelLeaveRequest = {
  id: string;
  personnelId: string;
  personnelName: string;
  type: PersonnelLeaveType;
  startDate: string;
  endDate: string;
  reason?: string;
  status: PersonnelLeaveStatus;
  createdAt: string;
  decidedAt?: string;
  decidedBy?: string;
  decisionNote?: string;
};

export const PERSONNEL_LEAVE_TYPE_LABELS: Record<PersonnelLeaveType, string> = {
  conge_paye: "Congé payé",
  rtt: "RTT",
  sans_solde: "Sans solde",
  autre: "Autre",
};

const PERSONNEL_LEAVE_STATUS_LABELS: Record<PersonnelLeaveStatus, string> = {
  en_attente: "En attente",
  validee: "Validée",
  refusee: "Refusée",
  annulee: "Annulée",
};

export type PersonnelOnboarding = {
  id: string;
  status: OnboardingStatus;
  startDate: string;
  checklist: Array<{ id: string; label: string; done: boolean }>;
  signatures: OnboardingSignature[];
  notes?: string;
};

export type PersonnelRecord = {
  id: string;
  externalUserId?: string | null;
  /** Email principal (pro de préférence) — compat modules existants */
  email: string;
  /** Email personnel */
  emailPerso?: string;
  /** Email professionnel / établissement */
  emailPro?: string;
  firstName: string;
  lastName: string;
  displayName: string;
  category: PersonnelCategory;
  jobTitle?: string;
  hireDate?: string | null;
  active: boolean;
  createdAt: string;
  updatedAt: string;
  documents: PersonnelDocument[];
  formations: PersonnelFormation[];
  habilitations: PersonnelHabilitation[];
  medecineTravail: PersonnelMedecineTravail;
  entretiens: PersonnelEntretien[];
  onboarding?: PersonnelOnboarding | null;
  offboarding?: PersonnelOffboarding | null;
  profile?: PersonnelProfile;
  establishment?: string | null;
  managerId?: string | null;
};

export type PersonnelIndexEntry = {
  id: string;
  displayName: string;
  email: string;
  emailPerso?: string;
  emailPro?: string;
  category: PersonnelCategory;
  externalUserId?: string | null;
  active: boolean;
  hireDate?: string | null;
  onboardingStatus?: OnboardingStatus | null;
};

export type SharedPersonnelDocument = {
  id: string;
  name: string;
  fileUrl: string;
  uploadedAt: string;
  uploadedBy: string;
};

export const PERSONNEL_INDEX_KEY = "personnel-ogec/index.json";
export const PERSONNEL_SHARED_DOCS_KEY = "personnel-ogec/shared-documents.json";
export const PERSONNEL_LEAVES_KEY = "personnel-ogec/leave-requests.json";
export const PERSONNEL_SIGNATURE_TOKEN_PREFIX = "personnel-ogec/signature-tokens/";

const OGEC_STAFF_ROLES = [
  "administratif",
  "maintenance",
  "surveillant",
  "cpe",
  "comptabilite",
] as const;

export const PERSONNEL_CATEGORY_LABELS: Record<PersonnelCategory, string> = {
  administratif: "Administratif",
  maintenance: "Maintenance",
  surveillant: "Surveillant",
  cpe: "CPE",
  comptabilite: "Comptabilité",
};

export function personnelRecordKey(id: string) {
  return `personnel-ogec/${id}.json`;
}

export function uid(prefix = "p") {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

function getPersonnelRoleFlags(roles: string[]) {
  return {
    isCompta: hasRole(roles, "comptabilite"),
    isAdministratif: hasRole(roles, "administratif"),
    isMaintenance: hasRole(roles, "maintenance"),
    isEducation: hasRole(roles, "surveillant"),
    isCpe: hasRole(roles, "cpe"),
    isDirection: isAnyDirectionRole(roles),
    isOgecStaff: OGEC_STAFF_ROLES.some((r) => hasRole(roles, r)) || isAnyDirectionRole(roles),
    isTeacher: hasRole(roles, "professeur"),
  };
}

/** RH / compta : gestion complète des dossiers OGEC. */
export function canManagePersonnel(_roles: string[]) {
  if (PERSONNEL_OPEN_ACCESS) return true;
  if (getPersonnelRoleFlags(_roles).isTeacher && !canViewPersonnelDashboard(_roles)) return false;
  return (
    hasRole(_roles, "comptabilite") ||
    hasRole(_roles, "administratif") ||
    hasRole(_roles, "admin")
  );
}

/** Tableau de bord RH + lecture de tous les dossiers. */
export function canViewPersonnelDashboard(_roles: string[]) {
  if (PERSONNEL_OPEN_ACCESS) return true;
  const f = getPersonnelRoleFlags(_roles);
  return (
    canManagePersonnel(_roles) || f.isDirection
  );
}

export function canAccessPersonnelModule(_roles: string[]) {
  if (PERSONNEL_OPEN_ACCESS) return true;
  const f = getPersonnelRoleFlags(_roles);
  return canViewPersonnelDashboard(_roles) || f.isOgecStaff;
}

export function inferCategoryFromRoles(roles: string[]): PersonnelCategory | null {
  if (hasRole(roles, "comptabilite")) return "comptabilite";
  if (hasRole(roles, "maintenance")) return "maintenance";
  if (hasRole(roles, "cpe")) return "cpe";
  if (hasRole(roles, "surveillant")) return "surveillant";
  if (hasRole(roles, "administratif")) return "administratif";
  if (
    hasRole(roles, "direction_ecole") ||
    hasRole(roles, "direction_college") ||
    hasRole(roles, "direction_lycee")
  ) {
    return "administratif";
  }
  return null;
}

export function canViewRecord(
  roles: string[],
  record: PersonnelRecord,
  viewerUserId?: string | null,
  viewerEmail?: string | null,
) {
  if (canViewPersonnelDashboard(roles)) return true;
  if (record.externalUserId && viewerUserId && record.externalUserId === viewerUserId) return true;
  const email = (viewerEmail || "").trim().toLowerCase();
  if (email && record.email.trim().toLowerCase() === email) return true;
  return false;
}

function filterDocumentsForViewer(
  docs: PersonnelDocument[],
  roles: string[],
  record: PersonnelRecord,
  viewerUserId?: string | null,
  viewerEmail?: string | null,
): PersonnelDocument[] {
  const isRh = canViewPersonnelDashboard(roles);
  const isSelf =
    (record.externalUserId && viewerUserId && record.externalUserId === viewerUserId) ||
    (viewerEmail && record.email.trim().toLowerCase() === viewerEmail.trim().toLowerCase());

  return docs.filter((d) => {
    if (d.visibility === "personnel") return isRh || isSelf;
    if (d.visibility === "establishment") return isRh;
    if (d.visibility === "restricted") return canManagePersonnel(roles);
    return isRh;
  });
}

export function sanitizeRecordForViewer(
  record: PersonnelRecord,
  roles: string[],
  viewerUserId?: string | null,
  viewerEmail?: string | null,
): PersonnelRecord {
  const isRh = canViewPersonnelDashboard(roles);
  const isSelf = canViewRecord(roles, record, viewerUserId, viewerEmail) && !isRh;

  return {
    ...record,
    documents: filterDocumentsForViewer(record.documents, roles, record, viewerUserId, viewerEmail),
    formations: isRh || isSelf ? record.formations : [],
    habilitations: isRh || isSelf ? record.habilitations : [],
    medecineTravail: isRh ? record.medecineTravail : isSelf ? { ...record.medecineTravail, notes: undefined } : {},
    entretiens: isRh ? record.entretiens : isSelf ? record.entretiens.filter((e) => e.status === "realise") : [],
    onboarding: isRh ? record.onboarding : null,
    offboarding: isRh ? record.offboarding : null,
  };
}

export function defaultOffboarding(endDate: string): PersonnelOffboarding {
  return {
    id: uid("off"),
    status: "en_cours",
    endDate,
    checklist: [
      { id: uid("chk"), label: "Restitution badge et clés", done: false },
      { id: uid("chk"), label: "Restitution matériel informatique", done: false },
      { id: uid("chk"), label: "Documents de fin de contrat remis", done: false },
      { id: uid("chk"), label: "Solde de tout compte / attestations", done: false },
    ],
    signatures: [
      { id: uid("sig"), role: "employe", label: "Employé(e)", status: "en_attente" },
      { id: uid("sig"), role: "directrice", label: "Direction", status: "en_attente" },
      { id: uid("sig"), role: "compta", label: "Comptabilité", status: "en_attente" },
    ],
  };
}

export function defaultOnboarding(startDate: string): PersonnelOnboarding {
  return {
    id: uid("onb"),
    status: "en_cours",
    startDate,
    checklist: [
      { id: uid("chk"), label: "Contrat de travail généré", done: false },
      { id: uid("chk"), label: "Déclaration prévoyance", done: false },
      { id: uid("chk"), label: "Remise règlement intérieur", done: false },
      { id: uid("chk"), label: "Badge et clés", done: false },
      { id: uid("chk"), label: "Formation sécurité / accueil", done: false },
    ],
    signatures: [
      { id: uid("sig"), role: "employe", label: "Employé(e)", status: "en_attente" },
      { id: uid("sig"), role: "directrice", label: "Directrice", status: "en_attente" },
      { id: uid("sig"), role: "compta", label: "Comptabilité", status: "en_attente" },
      { id: uid("sig"), role: "president_ogec", label: "Président OGEC", status: "en_attente" },
    ],
  };
}

export function defaultMedecineTravail(): PersonnelMedecineTravail {
  return { visits: [], lastVisitAt: null, nextVisitAt: null, visitType: "", notes: "" };
}

export function normalizePersonnelRecord(raw: unknown): PersonnelRecord {
  const o = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  const str = (v: unknown, fb = "") => (typeof v === "string" ? v : fb);
  const id = str(o.id).trim() || uid("p");
  const firstName = str(o.firstName).trim();
  const lastName = str(o.lastName).trim();
  const category = (
    ["administratif", "maintenance", "surveillant", "cpe", "comptabilite"] as const
  ).includes(o.category as PersonnelCategory)
    ? (o.category as PersonnelCategory)
    : "administratif";

  const docs = Array.isArray(o.documents) ? (o.documents as PersonnelDocument[]) : [];
  const formations = Array.isArray(o.formations) ? (o.formations as PersonnelFormation[]) : [];
  const habilitations = Array.isArray(o.habilitations) ? (o.habilitations as PersonnelHabilitation[]) : [];
  const entretiens = Array.isArray(o.entretiens) ? (o.entretiens as PersonnelEntretien[]) : [];
  const med =
    o.medecineTravail && typeof o.medecineTravail === "object"
      ? (o.medecineTravail as PersonnelMedecineTravail)
      : defaultMedecineTravail();

  const emailPerso = str(o.emailPerso).trim().toLowerCase() || undefined;
  const emailPro = str(o.emailPro).trim().toLowerCase() || undefined;
  const legacyEmail = str(o.email).trim().toLowerCase();
  const email = emailPro || emailPerso || legacyEmail;

  return {
    id,
    externalUserId: str(o.externalUserId) || null,
    email,
    emailPerso: emailPerso || (legacyEmail && !emailPro ? legacyEmail : undefined),
    emailPro,
    firstName,
    lastName,
    displayName: str(o.displayName).trim() || `${firstName} ${lastName}`.trim(),
    category,
    jobTitle: str(o.jobTitle) || undefined,
    hireDate: str(o.hireDate) || null,
    active: o.active !== false,
    createdAt: str(o.createdAt) || new Date().toISOString(),
    updatedAt: str(o.updatedAt) || new Date().toISOString(),
    documents: docs,
    formations,
    habilitations,
    medecineTravail: med,
    entretiens,
    onboarding: (o.onboarding as PersonnelOnboarding | null) || null,
    offboarding: (o.offboarding as PersonnelOffboarding | null) || null,
    profile: normalizePersonnelProfile(o.profile),
    establishment: str(o.establishment) || null,
    managerId: str(o.managerId) || null,
  };
}

export const PERSONNEL_CATEGORY_OPTIONS: { value: PersonnelCategory; label: string }[] = [
  { value: "administratif", label: "Administratif" },
  { value: "cpe", label: "CPE" },
  { value: "surveillant", label: "Surveillant" },
  { value: "comptabilite", label: "Comptabilité" },
  { value: "maintenance", label: "Maintenance" },
];

export function toIndexEntry(record: PersonnelRecord): PersonnelIndexEntry {
  return {
    id: record.id,
    displayName: record.displayName,
    email: record.email,
    emailPerso: record.emailPerso,
    emailPro: record.emailPro,
    category: record.category,
    externalUserId: record.externalUserId,
    active: record.active,
    hireDate: record.hireDate,
    onboardingStatus: record.onboarding?.status ?? null,
  };
}

export function daysUntil(dateIso: string | null | undefined): number | null {
  if (!dateIso) return null;
  const d = new Date(dateIso);
  if (Number.isNaN(d.getTime())) return null;
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  d.setHours(0, 0, 0, 0);
  return Math.ceil((d.getTime() - today.getTime()) / (1000 * 60 * 60 * 24));
}

export function isExpiringWithinDays(dateIso: string | null | undefined, withinDays: number) {
  const d = daysUntil(dateIso);
  return d !== null && d >= 0 && d <= withinDays;
}

export function isOverdue(dateIso: string | null | undefined) {
  const d = daysUntil(dateIso);
  return d !== null && d < 0;
}
