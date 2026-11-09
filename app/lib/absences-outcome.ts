/**
 * Issue lisible d'une absence : décision (qui, quand) et clôture (qui, quand, comment).
 * Sert à la coche du calendrier et à l'onglet « Suivi des absences traitées ».
 */
import {
  describeAbsenceValidationSuivi,
  formatAbsenceSuiviDateTime,
  type AbsenceHistoryEntry,
  type AbsenceSuiviSource,
} from "@/app/lib/absences-suivi";

export type AbsenceOutcomeSource = AbsenceSuiviSource & {
  workflowStatus?: string | null;
  closedAt?: string | null;
  adminTreatedAt?: string | null;
  adminTreatedBy?: string | null;
  adminNote?: string | null;
};

export type AbsenceOutcome = {
  state: "validee" | "refusee" | "en_attente";
  closed: boolean;
  /** Libellé court (badge). */
  shortLabel: string;
  /** « Validée par X le … » / « Refusée par X le … » / « En attente de validation ». */
  decisionLine: string;
  /** Comment le dossier s'est terminé (ou où il en est). */
  closureLine: string | null;
  /** Texte complet pour une info-bulle. */
  tooltip: string;
};

function lastEntry(history: AbsenceHistoryEntry[] | null | undefined, actions: string[]) {
  if (!Array.isArray(history)) return null;
  for (let i = history.length - 1; i >= 0; i -= 1) {
    if (actions.includes(history[i]?.action)) return history[i];
  }
  return null;
}

function whoWhen(prefix: string, by: string | null | undefined, at: string | null | undefined) {
  const who = String(by || "").trim();
  const when = at ? formatAbsenceSuiviDateTime(at) : "";
  if (who && when) return `${prefix} par ${who} le ${when}`;
  if (who) return `${prefix} par ${who}`;
  if (when) return `${prefix} le ${when}`;
  return prefix;
}

export function describeAbsenceOutcome(item: AbsenceOutcomeSource): AbsenceOutcome {
  const closed = item.workflowStatus === "CLOTUREE";

  if (item.managerDecision === "REFUSEE") {
    const ev = lastEntry(item.history, ["DECISION_REFUSEE"]);
    const decisionLine = whoWhen("Refusée", ev?.by, ev?.at);
    const closureLine = ev?.note?.trim() ? `Motif : ${ev.note.trim()}` : null;
    return {
      state: "refusee",
      closed,
      shortLabel: "Refusée",
      decisionLine,
      closureLine,
      tooltip: [decisionLine, closureLine].filter(Boolean).join(" — "),
    };
  }

  if (item.managerDecision !== "VALIDEE") {
    return {
      state: "en_attente",
      closed,
      shortLabel: "En attente",
      decisionLine: "En attente de validation",
      closureLine: null,
      tooltip: "En attente de validation",
    };
  }

  const validation = describeAbsenceValidationSuivi(item);
  const decisionLine = validation?.detail.split(" — ")[0] || "Validée";

  let closureLine: string;
  if (closed) {
    const ev = lastEntry(item.history, ["TRAITEMENT_ADMIN"]);
    const by = item.adminTreatedBy || ev?.by || null;
    const at = item.adminTreatedAt || item.closedAt || ev?.at || null;
    const note = String(item.adminNote || ev?.note || "").trim();
    const auto = /cl[oô]ture automatique/i.test(note);
    if (auto) {
      closureLine = `Dossier clôturé à la validation — ${note.replace(/^cl[oô]ture automatique\s*[—–-]\s*/i, "")}`;
    } else {
      closureLine = whoWhen("Clôturée", by, at) + (note ? ` — ${note}` : "");
    }
  } else {
    closureLine = "En attente du traitement RH / rectorat";
  }

  return {
    state: "validee",
    closed,
    shortLabel: closed ? "Validée · clôturée" : "Validée",
    decisionLine,
    closureLine,
    tooltip: `${decisionLine} — ${closureLine}`,
  };
}
