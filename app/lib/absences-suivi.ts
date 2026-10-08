/**
 * Suivi lisible d’une absence RH : qui a validé (ou prise d’acte),
 * et qui a le dossier dans son panier de traitement.
 */
import {
  isNonDiscretionaryTreatment,
  type AbsenceHoursTreatment,
} from "@/app/lib/absence-hours-treatment";
import type { Establishment, NotificationsConfig } from "@/app/lib/app-config-schemas";
import {
  collectAbsenceProcessors,
  type AbsenceProcessorRef,
} from "@/app/lib/absences-validation-recipients";
import { matchEstablishment } from "@/app/lib/establishment-catalog";
import { inferEstablishmentKind } from "@/app/lib/establishment-visual";

export type AbsenceHistoryEntry = {
  at: string;
  by: string;
  action: string;
  note?: string;
};

export type AbsenceSuiviSource = {
  managerDecision?: "EN_ATTENTE" | "VALIDEE" | "REFUSEE" | string | null;
  hoursTreatment?: AbsenceHoursTreatment | string | null;
  history?: AbsenceHistoryEntry[] | null;
  data: {
    scope: "professeur" | "ogec" | string;
    etablissement?: string | null;
  };
  createdBy: {
    roles?: string[] | null;
  };
};

/** Date/heure Europe/Paris pour l’historique absences. */
export function formatAbsenceSuiviDateTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat("fr-FR", {
    timeZone: "Europe/Paris",
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(d);
}

export function findLastValidationHistory(
  history?: AbsenceHistoryEntry[] | null,
): AbsenceHistoryEntry | null {
  if (!Array.isArray(history) || history.length === 0) return null;
  for (let i = history.length - 1; i >= 0; i -= 1) {
    const entry = history[i];
    if (entry?.action === "DECISION_VALIDEE") return entry;
  }
  return null;
}

export type AbsenceValidationSuivi = {
  /** true = arrêt / enfant malade / congé (pas de refus, transmission traitement). */
  priseActe: boolean;
  by: string | null;
  at: string | null;
  atLabel: string | null;
  note: string | null;
  headline: string;
  detail: string;
};

/**
 * Qui a validé / pris acte, et quand.
 * Les motifs structurés (arrêt, enfant malade, congé) restent une « prise d’acte »
 * même si une entrée DECISION_VALIDEE existe (reclassement ou bouton Valider).
 */
export function describeAbsenceValidationSuivi(
  item: AbsenceSuiviSource,
): AbsenceValidationSuivi | null {
  if (item.managerDecision !== "VALIDEE") return null;
  const ev = findLastValidationHistory(item.history);
  const note = ev?.note?.trim() || null;
  const priseActe =
    isNonDiscretionaryTreatment(item.hoursTreatment) || /prise d['’]acte/i.test(note || "");
  const by = ev?.by?.trim() || null;
  const at = ev?.at || null;
  const atLabel = at ? formatAbsenceSuiviDateTime(at) : null;

  if (priseActe) {
    const whoWhen =
      by && atLabel
        ? `Enregistré par ${by} le ${atLabel}`
        : by
          ? `Enregistré par ${by}`
          : atLabel
            ? `Enregistré le ${atLabel}`
            : null;
    return {
      priseActe: true,
      by,
      at,
      atLabel,
      note,
      headline: "Passage direct au traitement (prise d’acte)",
      detail: whoWhen
        ? `${whoWhen} — pas de validation discrétionnaire (arrêt de travail / enfant malade / congé).`
        : "Motif structuré — transmis au traitement sans refus possible.",
    };
  }

  const detail =
    by && atLabel
      ? `Validée par ${by} le ${atLabel}`
      : by
        ? `Validée par ${by}`
        : atLabel
          ? `Validée le ${atLabel}`
          : "Validée (acteur non renseigné sur cet ancien dossier).";

  return {
    priseActe: false,
    by,
    at,
    atLabel,
    note,
    headline: "Validation direction",
    detail: note && !/prise d['’]acte/i.test(note) ? `${detail} — ${note}` : detail,
  };
}

export type AbsenceProcessorBasket = {
  basketLabel: string;
  people: AbsenceProcessorRef[];
  peopleLabels: string[];
  emptyHint: string;
};

function processorDisplayLabel(p: AbsenceProcessorRef): string {
  const label = String(p.label || "").trim();
  if (label) return label;
  return String(p.email || "").trim();
}

/** Libellé du panier + personnes configurées pour traiter ce dossier. */
export function describeAbsenceProcessorBasket(
  item: AbsenceSuiviSource,
  notifications: NotificationsConfig | null | undefined,
  establishments: Establishment[],
): AbsenceProcessorBasket {
  const scope = item.data.scope === "ogec" ? "ogec" : "professeur";
  if (!notifications) {
    return {
      basketLabel: scope === "ogec" ? "Panier RH / compta (OGEC)" : "Panier secrétariat / rectorat",
      people: [],
      peopleLabels: [],
      emptyHint: "Paramétrage absences indisponible.",
    };
  }

  const people = collectAbsenceProcessors(
    {
      data: {
        scope,
        etablissement: item.data.etablissement ?? null,
        startDate: "",
        endDate: "",
        startAt: "",
        endAt: "",
        reason: "",
        details: "",
      },
      createdBy: {
        userId: "",
        name: "",
        email: "",
        roles: Array.isArray(item.createdBy.roles) ? item.createdBy.roles.map(String) : [],
      },
    },
    notifications,
    establishments,
  );

  let basketLabel = "Panier traitement";
  if (scope === "ogec") {
    basketLabel = "Panier RH / compta (personnel OGEC)";
  } else {
    const est = matchEstablishment(establishments, item.data.etablissement);
    const kind = est
      ? inferEstablishmentKind(est)
      : inferEstablishmentKind({ label: item.data.etablissement || "" });
    if (kind === "ecole") basketLabel = "Panier ONISE / secrétariat — école";
    else if (kind === "college") basketLabel = "Panier rectorat / secrétariat — collège";
    else if (kind === "lycee") basketLabel = "Panier rectorat / secrétariat — lycée";
    else basketLabel = "Panier secrétariat / rectorat (professeurs)";
  }

  return {
    basketLabel,
    people,
    peopleLabels: people.map(processorDisplayLabel).filter(Boolean),
    emptyHint:
      "Personne n’est configurée pour ce panier (Absences → Paramétrage · Traitement).",
  };
}
