/** Module Stages — offres parents, préconventions élèves, conventions multi-signatures. */

export const STAGE_S3 = {
  offersIndex: "stages/offers-index.json",
  conventionsIndex: "stages/conventions-index.json",
  offer: (id: string) => `stages/offers/${id}.json`,
  convention: (id: string) => `stages/conventions/${id}.json`,
  conventionUpload: (conventionId: string, safeFileName: string) =>
    `stages/uploads/${conventionId}/${safeFileName}`,
  signToken: (token: string) => `stages/sign-tokens/${token}.json`,
  signCodeLookup: (email: string, code: string) =>
    `stages/sign-code-lookup/${email.toLowerCase().replace(/[^a-z0-9@._-]/g, "_")}_${code}.json`,
  studentToken: (token: string) => `stages/student-tokens/${token}.json`,
  offerCandidatureToken: (token: string) => `stages/offer-candidature-tokens/${token}.json`,
  offerApplications: (offerId: string) => `stages/offer-applications/${offerId}.json`,
  referentsConfig: (schoolYear: string) => `stages/referents/${schoolYear}.json`,
  periodsConfig: (schoolYear: string) => `stages/periods/${schoolYear}.json`,
  referentSignature: (externalUserId: string) => `stages/signatures/referents/${externalUserId}.png`,
} as const;

export type StageOfferKind = "pfmp" | "stage_observation" | "job_ete" | "autre";

export type StageOfferStatus = "pending" | "approved" | "rejected" | "filled" | "archived";

export type StageOffer = {
  id: string;
  kind: StageOfferKind;
  status: StageOfferStatus;
  schoolYear: string;
  submittedBy: { externalUserId: string; displayName: string; email: string };
  companyName: string;
  companyAddress?: string;
  companySiret?: string;
  sector?: string;
  description: string;
  positionsCount: number;
  targetLevels: string[];
  periodStart?: string;
  periodEnd?: string;
  contactName: string;
  contactEmail: string;
  contactPhone?: string;
  reviewedAt?: string;
  reviewedBy?: string;
  reviewNote?: string;
  /** Lien public candidature élève (généré à la validation). */
  candidatureToken?: string;
  createdAt: string;
  updatedAt: string;
};

export type StageOfferApplication = {
  id: string;
  offerId: string;
  conventionId: string;
  studentFirstName: string;
  studentLastName: string;
  studentClassName: string;
  studentLevel: string;
  createdAt: string;
};

export type StageOfferCandidatureTokenRef = {
  offerId: string;
  createdAt: string;
};

export type StageScheduleMode = "uniform_week" | "per_day";

/** 1 = lundi … 5 = vendredi (ISO) */
export type StageWeekday = 1 | 2 | 3 | 4 | 5;

export type StageDaySlot = {
  /** Date ISO (mode per_day) ou absent si mode uniforme */
  date?: string;
  /** Jour de la semaine (mode uniform_week) */
  weekday?: StageWeekday;
  morningStart?: string | null;
  morningEnd?: string | null;
  afternoonStart?: string | null;
  afternoonEnd?: string | null;
  /** Si false : une seule plage (morningStart–morningEnd ou fullDay) */
  hasLunchBreak: boolean;
  fullDayStart?: string | null;
  fullDayEnd?: string | null;
};

export type StageSchedule = {
  mode: StageScheduleMode;
  periodStart: string;
  periodEnd: string;
  days: StageDaySlot[];
};

export type StageInternshipKind = "pfmp" | "stage_observation" | "job_ete" | "autre";

export type StageConventionStatus =
  | "draft"
  | "preconvention_submitted"
  | "admin_review"
  | "admin_rejected"
  | "convention_deposited"
  | "convention_ready"
  | "signatures_pending"
  | "signed"
  | "cancelled"
  | "archived";

export type StageSignerRole =
  | "eleve"
  | "parent"
  | "parent_2"
  | "tuteur_entreprise"
  | "rh_entreprise"
  | "professeur_referent"
  | "direction"
  | "administratif";

export type StageSignatureStatus = "en_attente" | "signe" | "refuse";

export type StageSignature = {
  id: string;
  role: StageSignerRole;
  label: string;
  status: StageSignatureStatus;
  signToken?: string;
  /** Code à 6 chiffres envoyé par e-mail (alternative au lien). */
  signSecureCode?: string;
  signEmail?: string;
  signSentAt?: string;
  signedAt?: string;
  signedBy?: string;
};

export type StageSignCodeLookupRef = {
  token: string;
  conventionId: string;
  signatureId: string;
  createdAt: string;
};

export type StageStudentInfo = {
  firstName: string;
  lastName: string;
  className: string;
  level: string;
  email?: string;
  /** Responsable légal 1 (obligatoire à la soumission). */
  parent1Email?: string;
  /** Responsable légal 2 (obligatoire à la soumission). */
  parent2Email?: string;
  /** @deprecated Préférer parent1Email — conservé pour compatibilité. */
  parentEmail?: string;
};

export type StageCompanyInfo = {
  name: string;
  address: string;
  siret?: string;
  activity: string;
  tutorName: string;
  tutorEmail: string;
  tutorPhone?: string;
  rhEmail?: string;
};

export type StageConvention = {
  id: string;
  schoolYear: string;
  status: StageConventionStatus;
  internshipKind: StageInternshipKind;
  student: StageStudentInfo;
  /** Jeton pour que l'élève complète sa préconvention sans compte */
  studentAccessToken?: string;
  offerId?: string;
  company: StageCompanyInfo;
  schedule: StageSchedule;
  /** Période officielle configurée (ex. PFMP 1, semaine 1). */
  stagePeriodId?: string;
  /** Libellé affiché pour ce stage (ex. « PFMP 1 », « Semaine 2 »). */
  stageLabel?: string;
  teacherReferent: { name: string; email: string; userId?: string };
  /** E-mail signataire responsable légal 1. */
  parentSignerEmail?: string;
  /** E-mail signataire responsable légal 2. */
  parent2SignerEmail?: string;
  adminReview?: {
    at: string;
    by: string;
    byName: string;
    approved: boolean;
    note?: string;
  };
  signatures: StageSignature[];
  createdAt: string;
  updatedAt: string;
  createdBy: { role: "eleve" | "parent" | "staff"; userId?: string; name: string };
  history: Array<{ at: string; by: string; action: string; note?: string }>;
  /** Dépôt OneDrive dossier élève (flux IAM / OCR). */
  oneDriveFiling?: {
    filedAt: string;
    filedBy: string;
    folderPath: string;
    fileName: string;
    matchedFolderName?: string;
  } | null;
  /** Dépôt auto dans le dossier élève ENT (tiroir scolaire). */
  eleveDossierFiling?: {
    filedAt: string;
    filedBy: string;
    eleveId: string;
    documentId: string;
    s3Key: string;
    title: string;
  } | null;
  oneDriveFilingPending?: boolean;
  oneDriveFilingError?: string;
  eleveDossierFilingPending?: boolean;
  eleveDossierFilingError?: string;
  /** PDF déposé par l'élève (convention papier / déjà signée). */
  uploadedPdf?: {
    s3Key: string;
    fileName: string;
    uploadedAt: string;
  };
  /** Métadonnées extraction OCR / IA. */
  ocrMeta?: {
    extractedAt: string;
    matchedEleveIne?: string;
    matchScore?: number;
    raw?: Record<string, unknown>;
  };
};

export type StageOfferIndexEntry = {
  id: string;
  kind: StageOfferKind;
  status: StageOfferStatus;
  companyName: string;
  targetLevels: string[];
  schoolYear: string;
  createdAt: string;
};

export type StageConventionIndexEntry = {
  id: string;
  status: StageConventionStatus;
  studentName: string;
  className: string;
  level: string;
  companyName: string;
  internshipKind: StageInternshipKind;
  periodStart: string;
  periodEnd: string;
  schoolYear: string;
  updatedAt: string;
  stageLabel?: string;
  teacherReferentEmail?: string;
};

export type StageSignTokenRef = {
  conventionId: string;
  signatureId: string;
  role: StageSignerRole;
  createdAt: string;
};

export type StageStudentTokenRef = {
  conventionId: string;
  createdAt: string;
};

export function stageUid(prefix = "stg") {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 9)}`;
}

export function studentDossierKey(student: Pick<StageStudentInfo, "firstName" | "lastName" | "className">) {
  return `${student.lastName.trim().toLowerCase()}|${student.firstName.trim().toLowerCase()}|${student.className.trim().toLowerCase()}`;
}

export function currentStageSchoolYear(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 8) return `${y}-${y + 1}`;
  return `${y - 1}-${y}`;
}

export const STAGE_OFFER_KIND_LABELS: Record<StageOfferKind, string> = {
  pfmp: "PFMP / Stage en entreprise",
  stage_observation: "Stage d'observation",
  job_ete: "Job d'été",
  autre: "Autre proposition",
};

export const STAGE_CONVENTION_STATUS_LABELS: Record<StageConventionStatus, string> = {
  draft: "Brouillon préconvention",
  preconvention_submitted: "Préconvention déposée",
  admin_review: "En validation administratif",
  admin_rejected: "À corriger (administratif)",
  convention_deposited: "Convention déposée (PDF)",
  convention_ready: "Convention prête",
  signatures_pending: "Signatures en cours",
  signed: "Convention signée",
  cancelled: "Annulée",
  archived: "Archivée",
};

export const STAGE_SIGNER_ROLE_LABELS: Record<StageSignerRole, string> = {
  eleve: "Élève",
  parent: "Responsable légal 1",
  parent_2: "Responsable légal 2",
  tuteur_entreprise: "Tuteur en entreprise",
  rh_entreprise: "RH entreprise",
  professeur_referent: "Professeur référent",
  direction: "Direction",
  administratif: "Administratif",
};
