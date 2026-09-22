import type { AbsencePeriodType } from "@/app/lib/absence-period";
import {
  forcedHoursTreatmentForNonDiscretionaryAbsence,
  formatTransmissionSummary,
  isNonDiscretionaryAbsence,
  isNonDiscretionaryTreatment,
  isRattrapageTreatment,
  isRectoratDeclarationTreatment,
  needsMakeupSlotsFromStaff,
  nonDiscretionaryKindLabel,
  suggestHoursTreatmentFromPreference,
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
    congeExceptionnelCode?: string | null;
    congeExceptionnelJoursSuggeres?: number | null;
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
  messages?: Array<{
    id: string;
    at: string;
    userId: string;
    userName: string;
    roleLabel: string;
    text: string;
  }>;
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

export function validationConfirmMessage(
  item: AbsenceItem,
  hoursTreatment?: AbsenceHoursTreatment | string | null,
) {
  if (isNonDiscretionaryAbsence(item) || isNonDiscretionaryTreatment(hoursTreatment)) {
    const forced =
      (isNonDiscretionaryTreatment(hoursTreatment)
        ? hoursTreatment
        : null) || forcedHoursTreatmentForNonDiscretionaryAbsence(item);
    const kind = nonDiscretionaryKindLabel(forced);
    const dest =
      item.data.scope === "ogec"
        ? "La comptabilité / RH traite ensuite le dossier."
        : "Le secrétariat traite ensuite le dossier.";
    return `Valider cette absence (${kind}) ?\n\nTraitement des heures : déclaration administrative obligatoire (sans rattrapage). Pas de refus possible. Le calendrier est mis à jour. ${dest}`;
  }
  const base = "Valider cette absence ? La décision direction est définitive.";
  if (item.data.scope === "ogec") {
    return `${base}\n\nLe calendrier est mis à jour. La RH traite ensuite le dossier dans l’application (pièces, clôture).`;
  }
  if (isRattrapageTreatment(hoursTreatment)) {
    return `${base}\n\nLe calendrier absences professeurs est mis à jour. Rattrapage interne : pas de passage par la personne qui déclare au rectorat.`;
  }
  if (isRectoratDeclarationTreatment(hoursTreatment)) {
    return `${base}\n\nLe calendrier absences professeurs est mis à jour. La personne en charge du rectorat / de l’instance traite ensuite le dossier dans l’application.`;
  }
  return `${base}\n\nLe calendrier absences professeurs est mis à jour.`;
}

export function transmissionLabel(item: AbsenceItem) {
  if (itemDecision(item) !== "VALIDEE") return null;
  if (item.workflowStatus !== "CLOTUREE") {
    if (isNonDiscretionaryTreatment(item.hoursTreatment)) {
      return item.data.scope === "ogec"
        ? "Validée — traitement des heures (arrêt de travail / enfant malade / congé exceptionnel) en cours à la comptabilité / RH."
        : "Validée — traitement des heures (arrêt de travail / enfant malade / congé exceptionnel) en cours au secrétariat.";
    }
    if (item.data.scope === "ogec") {
      return "Validée par la direction — en traitement RH.";
    }
    if (isRattrapageTreatment(item.hoursTreatment)) {
      return "Validée par la direction — rattrapage interne (sans déclaration rectorat).";
    }
    return "Validée par la direction — en traitement rectorat / instance.";
  }
  if (item.adminTreatedAt) {
    if (isNonDiscretionaryTreatment(item.hoursTreatment)) {
      return item.data.scope === "ogec"
        ? "Traitée par la RH (arrêt de travail / enfant malade / congé exceptionnel)."
        : "Traitée administrativement (arrêt de travail / enfant malade / congé exceptionnel).";
    }
    return item.data.scope === "ogec"
      ? "Traitée par la RH."
      : isRattrapageTreatment(item.hoursTreatment)
        ? "Validée — rattrapage interne (clôturée, sans déclaration rectorat)."
        : "Traitée administrativement (rectorat / instance).";
  }
  return formatTransmissionSummary(item.data.scope, item.data.etablissement, item.hoursTreatment);
}

export function resolvedHoursTreatment(item: AbsenceItem, draft: Record<string, string>) {
  if (draft[item.id]) return draft[item.id];
  if (item.hoursTreatment) return item.hoursTreatment;
  return (
    suggestHoursTreatmentFromPreference(
      item.data.scope,
      item.data.etablissement,
      item.staffPreferredTreatment,
    ) ?? ""
  );
}
