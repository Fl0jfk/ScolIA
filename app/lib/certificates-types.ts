import type { EstablishmentKind } from "@/app/lib/app-config-schemas";
import { DIRECTION_ROLE_BY_KIND } from "@/app/lib/establishment-catalog";

export type CertificateProgramStatus = "draft" | "signing" | "completed";
export type StudentAwardStatus =
  | "draft"
  | "submitted"
  | "prof_signed"
  | "direction_signed"
  | "issued";

export type CertificateSecteur = EstablishmentKind;

export type CertificateHistoryEntry = {
  at: string;
  by: string;
  action: string;
  note?: string;
};

export type CertificateProgramIndexEntry = {
  id: string;
  title: string;
  schoolYear: string;
  ownerId: string;
  ownerName: string;
  collaboratorIds: string[];
  status: CertificateProgramStatus;
  updatedAt: string;
};

export type CertificateProgram = CertificateProgramIndexEntry & {
  createdAt: string;
  history: CertificateHistoryEntry[];
};

export type CertificateLine = {
  id: string;
  title: string;
  /** Date ou période libre (ex. « Mars 2026 », « Oct.–Déc. 2025 »). */
  period?: string;
  description: string;
  /** Ancien format (une seule ligne de texte). */
  text?: string;
  addedBy: string;
  addedByName: string;
  addedAt: string;
};

export type DesignatedSignatory = {
  externalUserId: string;
  name: string;
  designatedBy: string;
  designatedAt: string;
  status: "pending" | "signed";
  signedAt?: string;
};

export type DirectionSignature = {
  signedBy: string;
  signedByName: string;
  signedAt: string;
  level: CertificateSecteur;
};

export type StudentAward = {
  id: string;
  programId: string;
  programTitle: string;
  schoolYear: string;
  addedBy: string;
  addedByName: string;
  student: {
    key: string;
    ine?: string;
    nom: string;
    prenom: string;
    classe: string;
    secteur: CertificateSecteur;
  };
  lines: CertificateLine[];
  designatedSignatories: DesignatedSignatory[];
  status: StudentAwardStatus;
  directionSignature?: DirectionSignature;
  verificationToken: string;
  contentHash: string;
  pdfS3Key?: string;
  issuedAt?: string;
  createdAt: string;
  updatedAt: string;
};

export type CertificateVerifySnapshot = {
  token: string;
  awardId: string;
  programTitle: string;
  schoolYear: string;
  student: StudentAward["student"];
  lines: CertificateLine[];
  designatedSignatories: DesignatedSignatory[];
  directionSignature?: DirectionSignature;
  contentHash: string;
  issuedAt: string;
  tenantName?: string;
};

export const CERTIFICATE_S3 = {
  programsIndex: "certificates/programs-index.json",
  program: (id: string) => `certificates/programs/${id}.json`,
  award: (id: string) => `certificates/awards/${id}.json`,
  awardsIndex: "certificates/awards-index.json",
  pdf: (id: string) => `certificates/pdfs/${id}.pdf`,
  verify: (token: string) => `certificates/verify/${token}.json`,
  profSignature: (externalUserId: string) => `certificates/signatures/prof/${externalUserId}.png`,
} as const;

const CERTIFICATE_PROGRAM_STATUS_LABELS: Record<CertificateProgramStatus, string> = {
  draft: "En cours",
  signing: "Signatures",
  completed: "Terminé",
};

export const STUDENT_AWARD_STATUS_LABELS: Record<StudentAwardStatus, string> = {
  draft: "Brouillon",
  submitted: "Soumis",
  prof_signed: "Profs signés",
  direction_signed: "Direction signée",
  issued: "Émis",
};

export const CERTIFICATE_SECTEUR_LABELS: Record<CertificateSecteur, string> = {
  ecole: "École",
  college: "Collège",
  lycee: "Lycée",
  custom: "Établissement",
};

export const CERTIFICATE_DIRECTION_LABELS: Record<CertificateSecteur, string> = {
  ecole: "Direction de l'école",
  college: "Direction du collège",
  lycee: "Direction du lycée",
  custom: "Direction",
};

export const CERTIFICATE_DIRECTION_ROLE_BY_SECTEUR: Record<CertificateSecteur, string> =
  DIRECTION_ROLE_BY_KIND;

export function certificateUid(prefix = "cert") {
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 9)}`;
}

export function currentCertificateSchoolYear(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 8) return `${y}-${y + 1}`;
  return `${y - 1}-${y}`;
}
