import { inferEstablishmentKind } from "@/app/lib/establishment-visual";

export type OgecHoursTreatment = "RATTRAPAGE" | "DEDUCTION_SALAIRE";
export type ProfHoursTreatment = "RATTRAPAGE_INTERNE" | "DECLARATION_ONISE" | "DECLARATION_RECTORAT";
export type AbsenceHoursTreatment = OgecHoursTreatment | ProfHoursTreatment;

type AbsenceScope = "professeur" | "ogec";
type Etablissement = string | null;

const RATTRAPAGE_INTERNE_OPTION = {
  value: "RATTRAPAGE_INTERNE" as const,
  label: "Heures rattrapées en interne (sans déclaration instance)",
};

export function getHoursTreatmentOptions(scope: AbsenceScope, etablissement: Etablissement | null) {
  if (scope === "ogec") {
    return [
      { value: "RATTRAPAGE" as const, label: "Heures à rattraper" },
      { value: "DEDUCTION_SALAIRE" as const, label: "Heures déduites du salaire" },
    ];
  }
  if (inferEstablishmentKind({ label: etablissement || "" }) === "ecole") {
    return [
      RATTRAPAGE_INTERNE_OPTION,
      { value: "DECLARATION_ONISE" as const, label: "À déclarer auprès de l'ONISE (instance)" },
    ];
  }
  return [
    RATTRAPAGE_INTERNE_OPTION,
    { value: "DECLARATION_RECTORAT" as const, label: "À déclarer auprès du rectorat (instance)" },
  ];
}

function parseAbsenceHoursTreatment(value: unknown): AbsenceHoursTreatment | null {
  if (
    value === "RATTRAPAGE" ||
    value === "DEDUCTION_SALAIRE" ||
    value === "RATTRAPAGE_INTERNE" ||
    value === "DECLARATION_ONISE" ||
    value === "DECLARATION_RECTORAT"
  ) {
    return value;
  }
  return null;
}

export function validateHoursTreatmentForAbsence(
  scope: AbsenceScope,
  etablissement: Etablissement | null,
  value: unknown,
): { ok: true; treatment: AbsenceHoursTreatment } | { ok: false; error: string } {
  const treatment = parseAbsenceHoursTreatment(value);
  if (!treatment) {
    return { ok: false, error: "Merci de choisir le traitement de l'absence avant de valider." };
  }
  const allowed = getHoursTreatmentOptions(scope, etablissement).map((o) => o.value);
  if (!allowed.includes(treatment)) {
    return { ok: false, error: "Traitement de l'absence invalide pour ce type de déclaration." };
  }
  return { ok: true, treatment };
}

export function formatAbsenceHoursTreatment(value?: AbsenceHoursTreatment | null): string | null {
  if (value === "RATTRAPAGE") return "Heures à rattraper";
  if (value === "DEDUCTION_SALAIRE") return "Heures déduites du salaire";
  if (value === "RATTRAPAGE_INTERNE") return "Heures rattrapées en interne (sans déclaration instance)";
  if (value === "DECLARATION_ONISE") return "À déclarer auprès de l'ONISE (instance)";
  if (value === "DECLARATION_RECTORAT") return "À déclarer auprès du rectorat (instance)";
  return null;
}

/** Ligne dédiée aux e-mails compta / secrétariat. */
export function formatHoursTreatmentMailLine(
  treatment: AbsenceHoursTreatment,
  scope: AbsenceScope,
): string {
  if (scope === "ogec") {
    if (treatment === "RATTRAPAGE") return "Décision de la direction : les heures seront rattrapées.";
    if (treatment === "DEDUCTION_SALAIRE") return "Décision de la direction : les heures seront déduites du salaire.";
  } else {
    if (treatment === "RATTRAPAGE_INTERNE") {
      return "Décision de la direction : les heures seront rattrapées en interne (sans déclaration auprès de l'instance).";
    }
    if (treatment === "DECLARATION_ONISE") return "À déclarer auprès de l'ONISE (instance).";
    if (treatment === "DECLARATION_RECTORAT") return "À déclarer auprès du rectorat (instance).";
  }
  return "";
}

/** Ligne pour l'e-mail de confirmation au demandeur. */
export function formatHoursTreatmentCreatorMailLine(
  treatment: AbsenceHoursTreatment,
  scope: AbsenceScope,
): string {
  if (scope === "ogec") {
    if (treatment === "RATTRAPAGE") return "Les heures d'absence seront rattrapées.";
    if (treatment === "DEDUCTION_SALAIRE") return "Les heures d'absence seront déduites du salaire.";
  } else {
    if (treatment === "RATTRAPAGE_INTERNE") {
      return "Les heures d'absence seront rattrapées en interne (sans déclaration auprès de l'instance).";
    }
    if (treatment === "DECLARATION_ONISE") {
      return "L'absence sera déclarée auprès de l'ONISE (instance) par le secrétariat.";
    }
    if (treatment === "DECLARATION_RECTORAT") {
      return "L'absence sera déclarée auprès du rectorat (instance) par le secrétariat.";
    }
  }
  return "";
}

export function hoursTreatmentFieldLabel(scope: AbsenceScope) {
  return scope === "ogec" ? "Traitement des heures" : "Traitement de l'absence";
}

export function formatTransmissionSummary(
  scope: AbsenceScope,
  etablissement: Etablissement | null,
  treatment?: AbsenceHoursTreatment | null,
): string | null {
  if (scope === "ogec") return "Transmise à la comptabilité.";
  if (treatment === "RATTRAPAGE_INTERNE") {
    return "Transmise au secrétariat — heures rattrapées en interne, sans déclaration instance.";
  }
  if (treatment === "DECLARATION_ONISE") return "Transmise au secrétariat — déclaration ONISE.";
  if (treatment === "DECLARATION_RECTORAT") return "Transmise au secrétariat — déclaration rectorat.";
  if (inferEstablishmentKind({ label: etablissement || "" }) === "ecole") {
    return "Transmise au secrétariat — déclaration ONISE.";
  }
  return "Transmise au secrétariat — déclaration rectorat.";
}
