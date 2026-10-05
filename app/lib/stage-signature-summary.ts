import {
  STAGE_CONVENTION_STATUS_LABELS,
  STAGE_SIGNER_ROLE_LABELS,
  conventionAllSignaturesValidated,
  isParentStageSignerRole,
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
  /**
   * Parent encore en attente alors que l'autre a déjà signé ET que tous les
   * autres signataires ont validé : dernière signature optionnelle (skippable).
   * Tant que d'autres signataires sont en cours, les deux parents restent actifs.
   */
  nonBlocking?: boolean;
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

function parentSiblingAlreadySigned(
  sig: StageSignature,
  all: StageSignature[],
): boolean {
  if (!isParentStageSignerRole(sig.role)) return false;
  return all.some(
    (other) =>
      other.id !== sig.id &&
      isParentStageSignerRole(other.role) &&
      isStageSignatureFullyValidated(other),
  );
}

/** True si tous les signataires hors parents ont déjà validé. */
function allNonParentSignaturesValidated(all: StageSignature[]): boolean {
  const others = all.filter((s) => !isParentStageSignerRole(s.role));
  if (others.length === 0) return true;
  return others.every(isStageSignatureFullyValidated);
}

function mapSignature(
  sig: StageSignature,
  all: StageSignature[],
): StageSignatureProgressItem {
  // Skip du 2ᵉ parent uniquement quand il serait la dernière signature attendue
  // (l'autre parent a signé + tout le reste est OK). Sinon les deux restent actifs.
  const nonBlocking =
    !isStageSignatureFullyValidated(sig) &&
    sig.status !== "refuse" &&
    parentSiblingAlreadySigned(sig, all) &&
    allNonParentSignaturesValidated(all);

  return {
    id: sig.id,
    role: sig.role,
    label: sig.label || STAGE_SIGNER_ROLE_LABELS[sig.role],
    status: sig.status,
    signedAt: sig.signedAt,
    reviewStatus: sig.reviewStatus,
    signMethod: sig.signMethod,
    nonBlocking: nonBlocking || undefined,
  };
}

/**
 * Compte chaque signature individuellement (affichage 4/5, etc.).
 * La règle métier « un seul parent suffit pour clôturer » reste dans
 * `conventionAllSignaturesValidated` / `complete`, pas dans ce ratio.
 */
function countSignatureProgress(signatures: StageSignature[]): {
  total: number;
  signed: number;
  pending: number;
  refused: number;
} {
  let signed = 0;
  let pending = 0;
  let refused = 0;
  for (const sig of signatures) {
    if (isStageSignatureFullyValidated(sig)) signed += 1;
    else if (sig.status === "refuse") refused += 1;
    else pending += 1;
  }
  return { total: signatures.length, signed, pending, refused };
}

export function buildSignatureSummary(convention: StageConvention): StageSignatureSummary {
  const signatures = convention.signatures;
  const items = signatures.map((sig) => mapSignature(sig, signatures));
  const units = countSignatureProgress(signatures);
  return {
    total: units.total,
    signed: units.signed,
    pending: units.pending,
    refused: units.refused,
    // Import hors plateforme (PDF déjà signé, aucune signature e-sign) : complet.
    complete:
      convention.status === "signed" || conventionAllSignaturesValidated(signatures),
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
