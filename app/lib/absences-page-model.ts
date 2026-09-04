import type { AbsencePeriodType } from "@/app/lib/absence-period";
import {
  formatTransmissionSummary,
  needsMakeupSlotsFromStaff,
  type AbsenceHoursTreatment,
} from "@/app/lib/absence-hours-treatment";

export { needsMakeupSlotsFromStaff };

export type AbsenceScope = "professeur" | "ogec";
export type Etablissement = string;
export type AbsenceWorkflowStatus = "OUVERTE" | "JUSTIFICATIF_DEPOSE" | "A_TRAITER" | "CLOTUREE";
export type AbsenceDecision = "EN_ATTENTE" | "VALIDEE" | "REFUSEE";
export type AbsenceItem = {
  id: string;
  createdAt: string;
  updatedAt: string;
  source?: string;
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
    reason: string;
    details: string;
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
  makeupSlotsRelanceAt?: string | null;
  adminTreatedAt?: string | null;
  adminTreatedBy?: string | null;
  adminNote?: string | null;
  staffPreferredTreatment?: string | null;
  staffPreferredMakeupSlots?: string | null;
  directionConfirmedMakeupSlots?: string | null;
};

export function itemDecision(item: AbsenceItem): AbsenceDecision {
  return item.managerDecision ?? "EN_ATTENTE";
}

export function isPendingAbsence(item: AbsenceItem) {
  return itemDecision(item) === "EN_ATTENTE" && item.workflowStatus !== "CLOTUREE";
}

export function isWaitingAdminTreatment(item: AbsenceItem) {
  return itemDecision(item) === "VALIDEE" && item.workflowStatus !== "CLOTUREE";
}

export function canDepositJustificatif(item: AbsenceItem) {
  if (item.workflowStatus === "CLOTUREE") return false;
  if (itemDecision(item) === "REFUSEE") return false;
  return (
    Boolean(item.justificatifRelanceAt) || isPendingAbsence(item) || isWaitingAdminTreatment(item)
  );
}

export function validationConfirmMessage(item: AbsenceItem) {
  const base = "Valider cette absence ? La décision direction est définitive.";
  if (item.data.scope === "ogec") {
    return `${base}\n\nLe calendrier est mis à jour. La RH traite ensuite le dossier dans l’application (pièces, clôture).`;
  }
  return `${base}\n\nLe calendrier absences professeurs est mis à jour. La personne en charge du rectorat / de l’instance traite ensuite le dossier dans l’application.`;
}

export function transmissionLabel(item: AbsenceItem) {
  if (itemDecision(item) !== "VALIDEE") return null;
  if (item.workflowStatus !== "CLOTUREE") {
    return item.data.scope === "ogec"
      ? "Validée par la direction — en traitement RH."
      : "Validée par la direction — en traitement rectorat / instance.";
  }
  if (item.adminTreatedAt) {
    return item.data.scope === "ogec"
      ? "Traitée par la RH."
      : "Traitée administrativement (rectorat / instance).";
  }
  return formatTransmissionSummary(item.data.scope, item.data.etablissement, item.hoursTreatment);
}

export function resolvedHoursTreatment(item: AbsenceItem, draft: Record<string, string>) {
  return draft[item.id] ?? item.hoursTreatment ?? "";
}
