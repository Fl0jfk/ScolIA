/**
 * Enrichissement appel ← occupancy (lot « UI lit le cerveau »).
 * Pur / testable : tags → libellés UI ; jamais fusionner en_sortie et absent_vs.
 */

import type { OccupancyFact, OccupancyTag } from "@/app/lib/occupancy/types";
import { isExcludedFromBulletinAbsence } from "@/app/lib/occupancy/types";

export type AppelOccupancyBadge = {
  eleveId: string;
  tag: OccupancyTag;
  labelFr: string;
  /** Si true : ne pas upsert absence bulletin (sortie). */
  excludeFromBulletin: boolean;
  /** Suggestion de statut ligne d’appel (préremplissage UI). */
  suggestedStatut: "present" | "absent" | "dispense" | null;
  detailFr?: string;
  source?: string;
};

export function occupancyTagToAppelBadge(
  fact: Pick<OccupancyFact, "eleveId" | "tag" | "source" | "detail">,
): AppelOccupancyBadge {
  const tag = fact.tag;
  const excludeFromBulletin = isExcludedFromBulletinAbsence(tag);

  switch (tag) {
    case "en_sortie":
      return {
        eleveId: fact.eleveId,
        tag,
        labelFr: "En sortie",
        excludeFromBulletin: true,
        suggestedStatut: "dispense",
        detailFr: "Participation voyage — pas une absence bulletin.",
        source: fact.source,
      };
    case "en_stage":
      return {
        eleveId: fact.eleveId,
        tag,
        labelFr: "En stage",
        excludeFromBulletin: true,
        suggestedStatut: "dispense",
        detailFr: fact.detail?.motif
          ? String(fact.detail.motif)
          : "Période de stage / PFMP — pas une absence bulletin.",
        source: fact.source,
      };
    case "a_infirmerie":
      return {
        eleveId: fact.eleveId,
        tag,
        labelFr: "À l’infirmerie",
        excludeFromBulletin: true,
        suggestedStatut: "dispense",
        detailFr: fact.detail?.motif
          ? String(fact.detail.motif)
          : "Passage infirmerie ouvert — pas une absence bulletin.",
        source: fact.source,
      };
    case "absent_vs":
      return {
        eleveId: fact.eleveId,
        tag,
        labelFr: "Déjà signalé absent",
        excludeFromBulletin: false,
        suggestedStatut: "absent",
        detailFr: fact.detail?.motif ? String(fact.detail.motif) : undefined,
        source: fact.source,
      };
    case "hors_etablissement":
      return {
        eleveId: fact.eleveId,
        tag,
        labelFr: "Hors établissement",
        excludeFromBulletin: false,
        suggestedStatut: "absent",
        detailFr: "Dernier passage portail = sortie.",
        source: fact.source,
      };
    case "internat":
      return {
        eleveId: fact.eleveId,
        tag,
        labelFr: "Internat",
        excludeFromBulletin: false,
        suggestedStatut: null,
        detailFr: "Signal internat (couverture partielle).",
        source: fact.source,
      };
    case "en_cours":
      return {
        eleveId: fact.eleveId,
        tag,
        labelFr: "Présumé en cours",
        excludeFromBulletin: false,
        suggestedStatut: null,
        source: fact.source,
      };
    default:
      return {
        eleveId: fact.eleveId,
        tag,
        labelFr: "Présence inconnue",
        excludeFromBulletin: false,
        suggestedStatut: null,
        source: fact.source,
      };
  }
}

export function badgesByEleveId(
  facts: Array<Pick<OccupancyFact, "eleveId" | "tag" | "source" | "detail">>,
): Map<string, AppelOccupancyBadge> {
  const map = new Map<string, AppelOccupancyBadge>();
  for (const f of facts) {
    // Ne pas polluer l’UI avec le défaut « en_cours » silencieux.
    if (f.tag === "en_cours") continue;
    map.set(f.eleveId, occupancyTagToAppelBadge(f));
  }
  return map;
}

export function mergeSuggestedStatut(opts: {
  existingStatut?: string | null;
  prevenuAccueil?: boolean;
  occupancy?: AppelOccupancyBadge | null;
}): "present" | "absent" | "retard" | "dispense" {
  if (opts.existingStatut === "present" || opts.existingStatut === "absent" || opts.existingStatut === "retard" || opts.existingStatut === "dispense") {
    return opts.existingStatut;
  }
  if (opts.occupancy?.suggestedStatut) return opts.occupancy.suggestedStatut;
  if (opts.prevenuAccueil) return "absent";
  return "present";
}
