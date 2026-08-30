import type { AbsencePeriodType } from "@/app/lib/absence-period";
import {
  formatTransmissionSummary,
  type AbsenceHoursTreatment,
} from "@/app/lib/absence-hours-treatment";

export type AbsenceScope = "professeur" | "ogec";
export type Etablissement = string;
export type AbsenceWorkflowStatus = "OUVERTE" | "JUSTIFICATIF_DEPOSE" | "CLOTUREE";
export type AbsenceDecision = "EN_ATTENTE" | "VALIDEE" | "REFUSEE";
export type AbsenceItem = {
  id: string;
  createdAt: string;
  updatedAt: string;
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
};

export function itemDecision(item: AbsenceItem): AbsenceDecision {
  return item.managerDecision ?? "EN_ATTENTE";
}

export function isPendingAbsence(item: AbsenceItem) {
  return itemDecision(item) === "EN_ATTENTE" && item.workflowStatus !== "CLOTUREE";
}

export function validationConfirmMessage(item: AbsenceItem) {
  const base =
    "Êtes-vous sûr de valider cette absence ?\n\nCette action est définitive (sans retour possible).";
  if (item.data.scope === "ogec") {
    return `${base}\n\nL'absence sera affichée au calendrier et transmise à la comptabilité RH.`;
  }
  return `${base}\n\nL'absence sera affichée au calendrier des absences professeurs et un e-mail partira à la personne qui gère les déclarations rectorat / instance (réglages Notifications).`;
}

export function transmissionLabel(item: AbsenceItem) {
  if (itemDecision(item) !== "VALIDEE") return null;
  return formatTransmissionSummary(item.data.scope, item.data.etablissement, item.hoursTreatment);
}

export function resolvedHoursTreatment(item: AbsenceItem, draft: Record<string, string>) {
  return draft[item.id] ?? item.hoursTreatment ?? "";
}
