/**
 * Résumé lisible d’une preview d’impacts voyage (confirm / cancel / tool).
 * Pur — utilisable côté client après la réponse API.
 */

import type { ImpactItem, VoyageImpactPreview } from "./types";

export type ImpactPreviewLike = Pick<
  VoyageImpactPreview,
  | "creneauxVides"
  | "wroteTeacherPlanningReplacement"
  | "participantCount"
  | "participantLinkedCount"
  | "impacts"
  | "status"
  | "title"
> & {
  creneauxVides?: Array<unknown> | null;
  impacts?: ImpactItem[] | null;
};

function tiroirLabel(t: string): string {
  switch (t) {
    case "A":
      return "A (auto)";
    case "B":
      return "B (signal)";
    case "C":
      return "C (question)";
    case "D":
      return "D (couverture)";
    default:
      return t;
  }
}

/** Lignes pour `alert()` / bandeau après confirm-liste ou annulation. */
export function formatImpactPreviewLines(preview: ImpactPreviewLike | null | undefined): string[] {
  if (!preview) return [];
  const lines: string[] = [];
  const creneaux = Array.isArray(preview.creneauxVides) ? preview.creneauxVides.length : 0;
  const linked = preview.participantLinkedCount ?? 0;
  const total = preview.participantCount ?? 0;

  if (total > 0) {
    lines.push(
      linked === total
        ? `${total} élève(s) lié(s) → occupancy « en sortie » sur les dates.`
        : `${linked}/${total} élève(s) lié(s) à un id live (occupancy partielle).`,
    );
  }

  if (creneaux > 0) {
    lines.push(
      `${creneaux} créneau(x) potentiellement vidé(s) — signal VS listé, pas de remplacement inventé.`,
    );
  } else if (preview.status !== "ANNULE" && preview.status !== "SEANCE_ANNULEE") {
    lines.push("Aucun créneau 100 % vidé détecté (EDT partiel ou classe non entièrement partie).");
  }

  if (preview.wroteTeacherPlanningReplacement === false) {
    lines.push("Planning prof : aucune écriture (conforme).");
  }

  const questions = (preview.impacts || []).filter((i) => i.tiroir === "C" && i.question);
  for (const q of questions.slice(0, 4)) {
    lines.push(`Question (${tiroirLabel(q.tiroir)}) : ${q.question}`);
  }
  if (questions.length > 4) {
    lines.push(`… +${questions.length - 4} autre(s) question(s).`);
  }

  return lines;
}

export function formatImpactPreviewAlert(preview: ImpactPreviewLike | null | undefined): string {
  const lines = formatImpactPreviewLines(preview);
  if (lines.length === 0) return "";
  return `Impacts (cerveau)\n${lines.join("\n")}`;
}
