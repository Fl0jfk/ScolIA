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

/** Libellé de la préférence exprimée par le déclarant (pas la décision direction). */
export function formatStaffPreferredTreatment(value?: string | null): string | null {
  if (!value) return null;
  if (value === "RATTRAPAGE" || value === "RATTRAPAGE_INTERNE") return "Rattrapage des heures";
  if (value === "DEDUCTION_SALAIRE") return "Déduction / perte de rémunération";
  if (value === "DECLARATION_INSTANCE" || value === "DECLARATION_ONISE" || value === "DECLARATION_RECTORAT") {
    return "Sans rattrapage (déclaration instance — impact service / rémunération)";
  }
  return value;
}

/** Lignes e-mail : préférence déclarant + créneaux confirmés direction. */
export function formatMakeupPreferenceMailLines(record: {
  staffPreferredTreatment?: string | null;
  staffPreferredMakeupSlots?: string | null;
  directionConfirmedMakeupSlots?: string | null;
  hoursTreatment?: AbsenceHoursTreatment | null;
}): string[] {
  const lines: string[] = [];
  const pref = formatStaffPreferredTreatment(record.staffPreferredTreatment);
  if (pref) lines.push(`Préférence du déclarant : ${pref}`);
  const preferredSlots = record.staffPreferredMakeupSlots?.trim();
  if (preferredSlots) lines.push(`Créneaux envisagés par le déclarant : ${preferredSlots}`);
  const confirmed = record.directionConfirmedMakeupSlots?.trim();
  if (confirmed) {
    lines.push(`Moment de rattrapage confirmé par la direction : ${confirmed}`);
  } else if (
    record.hoursTreatment === "RATTRAPAGE" ||
    record.hoursTreatment === "RATTRAPAGE_INTERNE"
  ) {
    lines.push(
      "Moment de rattrapage : à préciser par le déclarant dans l’application (relance envoyée si nécessaire).",
    );
  }
  return lines;
}

/** Un créneau de rattrapage saisi (jour + plage horaire). */
export type MakeupSlotDraft = {
  date: string;
  startTime: string;
  endTime: string;
};

export function emptyMakeupSlotDraft(): MakeupSlotDraft {
  return { date: "", startTime: "", endTime: "" };
}

function formatMakeupSlotDayFr(isoDate: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(isoDate.trim());
  if (!m) return isoDate.trim();
  const d = new Date(Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]), 12, 0, 0));
  if (Number.isNaN(d.getTime())) return isoDate.trim();
  return d.toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  });
}

function formatMakeupSlotTimeFr(hhmm: string): string {
  const raw = hhmm.trim();
  const m = /^(\d{1,2}):(\d{2})$/.exec(raw);
  if (!m) return raw;
  const h = Number(m[1]);
  const min = m[2];
  return min === "00" ? `${h}h` : `${h}h${min}`;
}

/** Transforme les créneaux structurés en texte lisible (stockage + e-mails). */
export function formatMakeupSlotsText(slots: MakeupSlotDraft[]): string {
  const lines = slots
    .map((slot) => {
      const date = slot.date.trim();
      const start = slot.startTime.trim();
      const end = slot.endTime.trim();
      if (!date || !start || !end) return null;
      return `${formatMakeupSlotDayFr(date)} de ${formatMakeupSlotTimeFr(start)} à ${formatMakeupSlotTimeFr(end)}`;
    })
    .filter((line): line is string => Boolean(line));
  return lines.join(" ; ");
}

export function isRattrapageTreatment(value?: string | null): boolean {
  return value === "RATTRAPAGE" || value === "RATTRAPAGE_INTERNE";
}

export function hasMakeupSlotsInfo(record: {
  staffPreferredMakeupSlots?: string | null;
  directionConfirmedMakeupSlots?: string | null;
}): boolean {
  return Boolean(
    record.staffPreferredMakeupSlots?.trim() || record.directionConfirmedMakeupSlots?.trim(),
  );
}

/** Absence en rattrapage sans moment indiqué → le déclarant doit préciser les créneaux. */
export function needsMakeupSlotsFromStaff(record: {
  workflowStatus?: string | null;
  managerDecision?: string | null;
  hoursTreatment?: string | null;
  staffPreferredTreatment?: string | null;
  staffPreferredMakeupSlots?: string | null;
  directionConfirmedMakeupSlots?: string | null;
  makeupSlotsRelanceAt?: string | null;
}): boolean {
  if (record.workflowStatus === "CLOTUREE") return false;
  if (record.managerDecision === "REFUSEE") return false;
  if (hasMakeupSlotsInfo(record)) return false;
  if (record.makeupSlotsRelanceAt) return true;
  if (record.managerDecision === "VALIDEE" && isRattrapageTreatment(record.hoursTreatment)) {
    return true;
  }
  if (
    record.managerDecision === "EN_ATTENTE" &&
    isRattrapageTreatment(record.staffPreferredTreatment)
  ) {
    return true;
  }
  return false;
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
