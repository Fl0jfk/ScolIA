import {
  STAGE_CONVENTION_STATUS_LABELS,
  STAGE_SIGNER_ROLE_LABELS,
  conventionAllSignaturesValidated,
  isStageSignatureFullyValidated,
  type StageConvention,
  type StageSignature,
  type StageSignatureStatus,
} from "@/app/lib/stage-types";

export type StageSignatureProgressItem = {
  id: string;
  role: string;
  label: string;
  status: StageSignatureStatus;
  signedAt?: string;
  reviewStatus?: StageSignature["reviewStatus"];
  signMethod?: StageSignature["signMethod"];
};

export type StageSignatureSummary = {
  total: number;
  signed: number;
  pending: number;
  refused: number;
  complete: boolean;
  items: StageSignatureProgressItem[];
};

export type StageConventionCard = {
  id: string;
  status: StageConvention["status"];
  statusLabel: string;
  stageLabel?: string;
  companyName: string;
  periodStart: string;
  periodEnd: string;
  updatedAt: string;
  studentAccessToken?: string;
  signatureSummary: StageSignatureSummary;
};

function mapSignature(sig: StageSignature): StageSignatureProgressItem {
  return {
    id: sig.id,
    role: sig.role,
    label: sig.label || STAGE_SIGNER_ROLE_LABELS[sig.role],
    status: sig.status,
    signedAt: sig.signedAt,
    reviewStatus: sig.reviewStatus,
    signMethod: sig.signMethod,
  };
}

export function buildSignatureSummary(convention: StageConvention): StageSignatureSummary {
  const items = convention.signatures.map(mapSignature);
  const signed = items.filter((s) => s.status === "signe").length;
  const pending = items.filter((s) => s.status === "en_attente").length;
  const refused = items.filter((s) => s.status === "refuse").length;
  const total = items.length;
  const validated = convention.signatures.filter(isStageSignatureFullyValidated).length;
  return {
    total,
    signed: validated,
    pending: total - validated - refused,
    refused,
    complete: conventionAllSignaturesValidated(convention.signatures),
    items,
  };
}

export function toConventionCard(convention: StageConvention): StageConventionCard {
  return {
    id: convention.id,
    status: convention.status,
    statusLabel: STAGE_CONVENTION_STATUS_LABELS[convention.status] || convention.status,
    stageLabel: convention.stageLabel,
    companyName: convention.company.name || "—",
    periodStart: convention.schedule.periodStart,
    periodEnd: convention.schedule.periodEnd,
    updatedAt: convention.updatedAt,
    studentAccessToken: convention.studentAccessToken,
    signatureSummary: buildSignatureSummary(convention),
  };
}

export function isActiveConventionStatus(status: StageConvention["status"]): boolean {
  return status !== "cancelled" && status !== "archived";
}
