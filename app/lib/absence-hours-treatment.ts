import { inferEstablishmentKind } from "@/app/lib/establishment-visual";

export type OgecHoursTreatment =
  | "RATTRAPAGE"
  | "DEDUCTION_SALAIRE"
  | "MALADIE"
  | "ENFANT_MALADE"
  | "CONGE_EXCEPTIONNEL";
export type ProfHoursTreatment =
  | "RATTRAPAGE_INTERNE"
  | "DECLARATION_ONISE"
  | "DECLARATION_RECTORAT"
  | "MALADIE"
  | "ENFANT_MALADE"
  | "CONGE_EXCEPTIONNEL";
export type AbsenceHoursTreatment = OgecHoursTreatment | ProfHoursTreatment;
export type NonDiscretionaryAbsenceTreatment = "MALADIE" | "ENFANT_MALADE" | "CONGE_EXCEPTIONNEL";

export type CongeExceptionnelSubmotifCode =
  | "MARIAGE_PACS"
  | "MARIAGE_ENFANT"
  | "NAISSANCE"
  | "ADOPTION"
  | "DECES_ENFANT_MOINS_25"
  | "DECES_ENFANT_25_PARENT"
  | "DECES_ENFANT_25"
  | "DECES_CONJOINT"
  | "DECES_PARENT"
  | "DECES_GRAND_PARENT"
  | "DECES_BEAU_PARENT"
  | "DECES_DESCENDANT"
  | "DECES_FRERE_SOEUR"
  | "ORDINATION"
  | "ANNONCE_HANDICAP_ENFANT"
  | "AUTRE";

export type CongeExceptionnelSubmotif = {
  code: CongeExceptionnelSubmotifCode;
  label: string;
  /**
   * Jours ouvrables suggérés = max(CCN EPNL art. 5.6.1, plancher Code du travail L. 3142-4).
   * null = à préciser (hors liste).
   */
  joursOuvrables: number | null;
  aide: string;
};

type AbsenceScope = "professeur" | "ogec";
type Etablissement = string | null;

const RATTRAPAGE_INTERNE_OPTION = {
  value: "RATTRAPAGE_INTERNE" as const,
  label: "Heures rattrapées en interne (sans déclaration instance)",
};

/** Motifs structurés : traitement des heures forcé (déclaré), sans rattrapage ; direction = validation seule (pas de refus). */
export const NON_DISCRETIONARY_ABSENCE_REASONS = [
  { value: "Arrêt de travail", treatment: "MALADIE" as const, label: "Arrêt de travail" },
  { value: "Enfant malade", treatment: "ENFANT_MALADE" as const, label: "Enfant malade" },
  {
    value: "Congé exceptionnel",
    treatment: "CONGE_EXCEPTIONNEL" as const,
    label: "Congé exceptionnel",
  },
] as const;

/**
 * Sous-motifs congé exceptionnel — CCN EPNL art. 5.6.1 (juil. 2025),
 * plafonnés au plancher légal L. 3142-4 lorsque la loi est plus favorable.
 * Durées indicatives ; jours ouvrables successifs (sauf accord employeur).
 */
export const CONGE_EXCEPTIONNEL_SUBMOTIFS: readonly CongeExceptionnelSubmotif[] = [
  {
    code: "MARIAGE_PACS",
    label: "Mariage ou PACS du salarié",
    joursOuvrables: 4,
    aide: "CCN EPNL : 4 jours ouvrables.",
  },
  {
    code: "NAISSANCE",
    label: "Naissance survenue au foyer",
    joursOuvrables: 3,
    aide: "CCN EPNL : 3 jours ouvrables (non cumulables avec maternité / paternité pour le même enfant).",
  },
  {
    code: "ADOPTION",
    label: "Adoption (arrivée d’un enfant placé)",
    joursOuvrables: 3,
    aide: "CCN EPNL : 3 jours ouvrables.",
  },
  {
    code: "MARIAGE_ENFANT",
    label: "Mariage ou PACS d’un enfant",
    joursOuvrables: 3,
    aide: "CCN EPNL : 3 jours ouvrables (plancher légal : 1 jour).",
  },
  {
    code: "DECES_ENFANT_MOINS_25",
    label: "Décès d’un enfant ou d’une personne à charge de moins de 25 ans",
    joursOuvrables: 14,
    aide: "Plancher légal : 14 jours ouvrables (CCN EPNL : 8). Congé de deuil complémentaire possible (8 j., hors ce circuit).",
  },
  {
    code: "DECES_ENFANT_25_PARENT",
    label: "Décès d’un enfant de 25 ans et plus qui était lui-même parent",
    joursOuvrables: 14,
    aide: "Plancher légal : 14 jours ouvrables (CCN EPNL : 8).",
  },
  {
    code: "DECES_ENFANT_25",
    label: "Décès d’un enfant de 25 ans et plus (sans être parent)",
    joursOuvrables: 12,
    aide: "Plancher légal : 12 jours ouvrables (CCN EPNL : 5).",
  },
  {
    code: "DECES_CONJOINT",
    label: "Décès du conjoint, partenaire de PACS ou concubin",
    joursOuvrables: 5,
    aide: "CCN EPNL : 5 jours ouvrables (plancher légal : 3).",
  },
  {
    code: "DECES_PARENT",
    label: "Décès d’un parent (père ou mère)",
    joursOuvrables: 5,
    aide: "CCN EPNL : 5 jours ouvrables (ascendant en ligne directe — parent).",
  },
  {
    code: "DECES_GRAND_PARENT",
    label: "Décès d’un grand-parent (ou ascendant au-delà)",
    joursOuvrables: 3,
    aide: "CCN EPNL : 3 jours ouvrables (non prévu par le Code du travail seul).",
  },
  {
    code: "DECES_BEAU_PARENT",
    label: "Décès d’un beau-parent",
    joursOuvrables: 3,
    aide: "CCN EPNL : 3 jours ouvrables (père ou mère du conjoint / concubin / partenaire de PACS).",
  },
  {
    code: "DECES_DESCENDANT",
    label: "Décès d’un descendant en ligne directe (autre qu’un enfant)",
    joursOuvrables: 3,
    aide: "CCN EPNL : 3 jours ouvrables.",
  },
  {
    code: "DECES_FRERE_SOEUR",
    label: "Décès d’un frère ou d’une sœur",
    joursOuvrables: 3,
    aide: "CCN EPNL : 3 jours ouvrables.",
  },
  {
    code: "ORDINATION",
    label: "Ordination diaconale / sacerdotale ou vœux perpétuels",
    joursOuvrables: 3,
    aide: "CCN EPNL : 3 jours ouvrables (salarié, conjoint ou enfant).",
  },
  {
    code: "ANNONCE_HANDICAP_ENFANT",
    label: "Annonce d’un handicap, pathologie chronique ou cancer chez un enfant",
    joursOuvrables: 10,
    aide: "Plancher légal : 10 jours ouvrables (CCN EPNL : 2 — la loi prime).",
  },
  {
    code: "AUTRE",
    label: "Autre motif familial (à préciser)",
    joursOuvrables: null,
    aide: "Hors liste CCN EPNL art. 5.6.1 — durée à préciser avec la direction.",
  },
] as const;

export function getCongeExceptionnelSubmotif(
  code?: string | null,
): CongeExceptionnelSubmotif | null {
  if (!code) return null;
  return CONGE_EXCEPTIONNEL_SUBMOTIFS.find((s) => s.code === code) ?? null;
}

export function formatCongeExceptionnelReason(
  code?: string | null,
  fallbackLabel?: string | null,
): string {
  const sub = getCongeExceptionnelSubmotif(code);
  if (sub) return `Congé exceptionnel — ${sub.label}`;
  if (fallbackLabel?.trim()) return fallbackLabel.trim();
  return "Congé exceptionnel";
}

/** Ajoute n jours ouvrables (lun–ven) à une date ISO YYYY-MM-DD. n = 1 → même jour. */
export function addOuvrableDays(isoDate: string, joursOuvrables: number): string | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(String(isoDate || "").trim());
  if (!m || !Number.isFinite(joursOuvrables) || joursOuvrables < 1) return null;
  const d = new Date(Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]), 12, 0, 0));
  if (Number.isNaN(d.getTime())) return null;
  let remaining = Math.floor(joursOuvrables);
  while (remaining > 0) {
    const day = d.getUTCDay(); // 0=dim, 6=sam
    if (day !== 0 && day !== 6) {
      remaining -= 1;
      if (remaining === 0) break;
    }
    d.setUTCDate(d.getUTCDate() + 1);
  }
  const y = d.getUTCFullYear();
  const mo = String(d.getUTCMonth() + 1).padStart(2, "0");
  const da = String(d.getUTCDate()).padStart(2, "0");
  return `${y}-${mo}-${da}`;
}

export function congeExceptionnelJoursAide(code?: string | null): string {
  const sub = getCongeExceptionnelSubmotif(code);
  if (!sub) {
    return "Durées indicatives (CCN EPNL art. 5.6.1, plancher Code du travail). Les dates restent libres.";
  }
  if (sub.joursOuvrables == null) {
    return `${sub.aide} Les dates restent libres.`;
  }
  return `${sub.aide} Indicatif — jours ouvrables successifs (sauf accord). Les dates restent libres.`;
}

export function isNonDiscretionaryTreatment(value?: string | null): boolean {
  return value === "MALADIE" || value === "ENFANT_MALADE" || value === "CONGE_EXCEPTIONNEL";
}

function normalizeReasonText(reason?: string | null): string {
  return String(reason || "")
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "");
}

export function nonDiscretionaryTreatmentFromReason(
  reason?: string | null,
): NonDiscretionaryAbsenceTreatment | null {
  const normalized = normalizeReasonText(reason);
  if (!normalized) return null;
  if (
    normalized === "enfant malade" ||
    normalized === "enfants malades" ||
    /\benfant[s]?\s+malade/.test(normalized)
  ) {
    return "ENFANT_MALADE";
  }
  if (
    normalized === "conge exceptionnel" ||
    normalized.startsWith("conge exceptionnel") ||
    normalized.startsWith("conges exceptionnels")
  ) {
    return "CONGE_EXCEPTIONNEL";
  }
  if (
    normalized === "maladie" ||
    normalized === "arret maladie" ||
    normalized === "arrets maladie" ||
    normalized === "arret de travail" ||
    normalized === "arrets de travail" ||
    /\barret[s]?\s*[- ]?\s*maladie\b/.test(normalized) ||
    /\barret[s]?\s+de\s+travail\b/.test(normalized)
  ) {
    return "MALADIE";
  }
  return null;
}

export function reasonLabelForNonDiscretionaryTreatment(
  treatment: NonDiscretionaryAbsenceTreatment,
  congeExceptionnelCode?: string | null,
): string {
  if (treatment === "MALADIE") return "Arrêt de travail";
  if (treatment === "ENFANT_MALADE") return "Enfant malade";
  return formatCongeExceptionnelReason(congeExceptionnelCode);
}

/** Absence arrêt de travail / enfant malade / congé exceptionnel : validation direction obligatoire, refus interdit. */
export function isNonDiscretionaryAbsence(record: {
  staffPreferredTreatment?: string | null;
  hoursTreatment?: string | null;
  data?: { reason?: string | null } | null;
}): boolean {
  if (isNonDiscretionaryTreatment(record.hoursTreatment)) return true;
  if (isNonDiscretionaryTreatment(record.staffPreferredTreatment)) return true;
  return Boolean(nonDiscretionaryTreatmentFromReason(record.data?.reason));
}

export function forcedHoursTreatmentForNonDiscretionaryAbsence(record: {
  staffPreferredTreatment?: string | null;
  hoursTreatment?: string | null;
  data?: { reason?: string | null } | null;
}): NonDiscretionaryAbsenceTreatment | null {
  if (
    record.hoursTreatment === "MALADIE" ||
    record.hoursTreatment === "ENFANT_MALADE" ||
    record.hoursTreatment === "CONGE_EXCEPTIONNEL"
  ) {
    return record.hoursTreatment;
  }
  if (
    record.staffPreferredTreatment === "MALADIE" ||
    record.staffPreferredTreatment === "ENFANT_MALADE" ||
    record.staffPreferredTreatment === "CONGE_EXCEPTIONNEL"
  ) {
    return record.staffPreferredTreatment;
  }
  return nonDiscretionaryTreatmentFromReason(record.data?.reason);
}

export function nonDiscretionaryKindLabel(
  treatment: NonDiscretionaryAbsenceTreatment | null | undefined,
): string {
  if (treatment === "ENFANT_MALADE") return "enfant malade";
  if (treatment === "CONGE_EXCEPTIONNEL") return "congé exceptionnel";
  return "arrêt de travail";
}

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
    value === "DECLARATION_RECTORAT" ||
    value === "MALADIE" ||
    value === "ENFANT_MALADE" ||
    value === "CONGE_EXCEPTIONNEL"
  ) {
    return value;
  }
  return null;
}

export function validateHoursTreatmentForAbsence(
  scope: AbsenceScope,
  etablissement: Etablissement | null,
  value: unknown,
  options?: { allowNonDiscretionary?: boolean },
): { ok: true; treatment: AbsenceHoursTreatment } | { ok: false; error: string } {
  const treatment = parseAbsenceHoursTreatment(value);
  if (!treatment) {
    return { ok: false, error: "Merci de choisir le traitement de l'absence avant de valider." };
  }
  if (isNonDiscretionaryTreatment(treatment)) {
    if (options?.allowNonDiscretionary) return { ok: true, treatment };
    return {
      ok: false,
      error:
        "Traitement arrêt de travail / enfant malade / congé exceptionnel réservé aux absences de ce type.",
    };
  }
  const allowed = getHoursTreatmentOptions(scope, etablissement).map((o) => o.value as string);
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
  if (value === "MALADIE") return "Arrêt de travail — heures à déclarer (sans rattrapage)";
  if (value === "ENFANT_MALADE") return "Enfant malade — heures à déclarer (sans rattrapage)";
  if (value === "CONGE_EXCEPTIONNEL") {
    return "Congé exceptionnel — heures à déclarer (sans rattrapage)";
  }
  return null;
}

/** Ligne dédiée aux e-mails compta / secrétariat. */
export function formatHoursTreatmentMailLine(
  treatment: AbsenceHoursTreatment,
  scope: AbsenceScope,
): string {
  if (treatment === "MALADIE") {
    return scope === "ogec"
      ? "Traitement des heures : arrêt de travail — à traiter en comptabilité (sans rattrapage)."
      : "Traitement des heures : arrêt de travail — à traiter au secrétariat (sans rattrapage).";
  }
  if (treatment === "ENFANT_MALADE") {
    return scope === "ogec"
      ? "Traitement des heures : enfant malade — à traiter en comptabilité (sans rattrapage)."
      : "Traitement des heures : enfant malade — à traiter au secrétariat (sans rattrapage).";
  }
  if (treatment === "CONGE_EXCEPTIONNEL") {
    return scope === "ogec"
      ? "Traitement des heures : congé exceptionnel — à traiter en comptabilité (sans rattrapage)."
      : "Traitement des heures : congé exceptionnel — à traiter au secrétariat (sans rattrapage).";
  }
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
  if (treatment === "MALADIE") {
    return "Votre arrêt de travail a été pris en compte par la direction. Le dossier est transmis pour traitement administratif.";
  }
  if (treatment === "ENFANT_MALADE") {
    return "Votre absence pour enfant malade a été prise en compte par la direction. Le dossier est transmis pour traitement administratif.";
  }
  if (treatment === "CONGE_EXCEPTIONNEL") {
    return "Votre congé exceptionnel a été pris en compte par la direction. Le dossier est transmis pour traitement administratif.";
  }
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

/** Préférence déclarant = sans rattrapage (déclaration instance / rectorat / ONISE). */
export function isDeclarationPreference(value?: string | null): boolean {
  return (
    value === "DECLARATION_INSTANCE" ||
    value === "DECLARATION_ONISE" ||
    value === "DECLARATION_RECTORAT"
  );
}

/** Décision direction professeurs qui nécessite un traitement secrétariat (rectorat / ONISE). */
export function isRectoratDeclarationTreatment(value?: string | null): boolean {
  return value === "DECLARATION_RECTORAT" || value === "DECLARATION_ONISE";
}

/**
 * Préremplit le choix direction à partir de la préférence du déclarant.
 * Professeurs : DECLARATION_INSTANCE → ONISE (école) ou rectorat (collège / lycée).
 * OGEC : mapping direct rattrapage / déduction.
 */
export function suggestHoursTreatmentFromPreference(
  scope: AbsenceScope,
  etablissement: Etablissement | null,
  staffPreferredTreatment?: string | null,
): AbsenceHoursTreatment | null {
  const pref = String(staffPreferredTreatment || "").trim();
  if (!pref) return null;
  if (pref === "MALADIE" || pref === "ENFANT_MALADE" || pref === "CONGE_EXCEPTIONNEL") return pref;
  if (scope === "ogec") {
    if (pref === "RATTRAPAGE" || pref === "DEDUCTION_SALAIRE") return pref;
    return null;
  }
  if (pref === "RATTRAPAGE_INTERNE" || pref === "RATTRAPAGE") return "RATTRAPAGE_INTERNE";
  if (isDeclarationPreference(pref)) {
    return inferEstablishmentKind({ label: etablissement || "" }) === "ecole"
      ? "DECLARATION_ONISE"
      : "DECLARATION_RECTORAT";
  }
  return null;
}

/** Libellé de la préférence exprimée par le déclarant (pas la décision direction). */
export function formatStaffPreferredTreatment(value?: string | null): string | null {
  if (!value) return null;
  if (value === "RATTRAPAGE" || value === "RATTRAPAGE_INTERNE") return "Rattrapage des heures";
  if (value === "DEDUCTION_SALAIRE") return "Déduction / perte de rémunération";
  if (value === "DECLARATION_INSTANCE" || value === "DECLARATION_ONISE" || value === "DECLARATION_RECTORAT") {
    return "Sans rattrapage (déclaration au rectorat / instance)";
  }
  if (value === "MALADIE") return "Arrêt de travail — heures à déclarer (sans rattrapage)";
  if (value === "ENFANT_MALADE") return "Enfant malade — heures à déclarer (sans rattrapage)";
  if (value === "CONGE_EXCEPTIONNEL") {
    return "Congé exceptionnel — heures à déclarer (sans rattrapage)";
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
  if (record.managerDecision === "REFUSEE") return false;
  // Relance active : le déclarant peut (re)préciser même s’il avait déjà proposé des créneaux.
  if (record.makeupSlotsRelanceAt && isRattrapageTreatment(record.hoursTreatment || record.staffPreferredTreatment)) {
    if (
      record.managerDecision === "VALIDEE" &&
      record.hoursTreatment &&
      !isRattrapageTreatment(record.hoursTreatment)
    ) {
      return false;
    }
    return true;
  }
  if (hasMakeupSlotsInfo(record)) return false;

  // Décision direction = déclaration / déduction : jamais de demande de créneaux.
  if (
    record.managerDecision === "VALIDEE" &&
    record.hoursTreatment &&
    !isRattrapageTreatment(record.hoursTreatment)
  ) {
    return false;
  }

  // Préférence explicite sans rattrapage (en attente direction) : pas de créneaux.
  if (
    record.managerDecision === "EN_ATTENTE" &&
    (isDeclarationPreference(record.staffPreferredTreatment) ||
      record.staffPreferredTreatment === "DEDUCTION_SALAIRE" ||
      isNonDiscretionaryTreatment(record.staffPreferredTreatment))
  ) {
    return false;
  }

  if (record.managerDecision === "VALIDEE" && isRattrapageTreatment(record.hoursTreatment)) {
    // Rattrapage prof clôturé auto : on peut encore demander les créneaux si relance active.
    if (record.workflowStatus === "CLOTUREE") {
      return Boolean(record.makeupSlotsRelanceAt);
    }
    return true;
  }

  if (record.workflowStatus === "CLOTUREE") return false;

  if (
    record.managerDecision === "EN_ATTENTE" &&
    isRattrapageTreatment(record.staffPreferredTreatment)
  ) {
    return true;
  }
  return false;
}

/**
 * Professeurs : secrétariat pour déclarations instance + motifs non discrétionnaires.
 * OGEC : tout dossier validé reste chez la RH / compta (flux distinct).
 */
export function requiresProcessorAfterValidation(record: {
  data?: { scope?: string | null } | null;
  hoursTreatment?: string | null;
}): boolean {
  const scope = record.data?.scope;
  if (scope === "ogec") return true;
  if (scope === "professeur") {
    return (
      isRectoratDeclarationTreatment(record.hoursTreatment) ||
      isNonDiscretionaryTreatment(record.hoursTreatment)
    );
  }
  return Boolean(record.hoursTreatment);
}

export function formatTransmissionSummary(
  scope: AbsenceScope,
  etablissement: Etablissement | null,
  treatment?: AbsenceHoursTreatment | null,
): string | null {
  if (treatment === "MALADIE") {
    return scope === "ogec"
      ? "Transmise à la comptabilité — arrêt de travail."
      : "Transmise au secrétariat — arrêt de travail.";
  }
  if (treatment === "ENFANT_MALADE") {
    return scope === "ogec"
      ? "Transmise à la comptabilité — enfant malade."
      : "Transmise au secrétariat — enfant malade.";
  }
  if (treatment === "CONGE_EXCEPTIONNEL") {
    return scope === "ogec"
      ? "Transmise à la comptabilité — congé exceptionnel."
      : "Transmise au secrétariat — congé exceptionnel.";
  }
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
