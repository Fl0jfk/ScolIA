import {
  isExcludedFromBulletinAbsence,
  tagPriorityIndex,
  type OccupancyCoverage,
  type OccupancyFact,
  type OccupancySignal,
  type OccupancyTag,
} from "./types";

/**
 * Fusionne les signaux d'un élève pour une date.
 * Priorité : en_sortie > en_stage > absent_vs > internat > en_cours > inconnu.
 */
export function mergeOccupancySignals(opts: {
  eleveId: string;
  date: string;
  signals: OccupancySignal[];
  defaultTag?: OccupancyTag;
}): OccupancyFact {
  const scoped = opts.signals.filter((s) => s.eleveId === opts.eleveId);
  if (scoped.length === 0) {
    const tag = opts.defaultTag ?? "inconnu";
    return {
      eleveId: opts.eleveId,
      date: opts.date,
      tag,
      source: tag === "en_cours" ? "scolarite" : "none",
      coverage: tag === "inconnu" ? "partial" : "complete",
      confidenceTag: tag === "inconnu" ? "unknown" : "deduced",
    };
  }

  let best = scoped[0]!;
  for (let i = 1; i < scoped.length; i++) {
    const cur = scoped[i]!;
    if (tagPriorityIndex(cur.tag) < tagPriorityIndex(best.tag)) best = cur;
  }

  return {
    eleveId: opts.eleveId,
    date: opts.date,
    tag: best.tag,
    source: best.source,
    coverage: best.coverage ?? "complete",
    confidenceTag: best.confidenceTag ?? "defined",
    detail: best.detail,
  };
}

export function mergeCoverage(parts: OccupancyCoverage[]): OccupancyCoverage {
  if (parts.length === 0) return "complete";
  if (parts.includes("unavailable")) return "unavailable";
  if (parts.includes("partial")) return "partial";
  return "complete";
}

/** Filtre pour l'agrégat bulletin / upsert absence appel : exclut `en_sortie`. */
export function filterBulletinEligibleEleveIds(
  eleveIds: string[],
  factsByEleve: Map<string, OccupancyFact>,
): string[] {
  return eleveIds.filter((id) => {
    const fact = factsByEleve.get(id);
    if (!fact) return true;
    return !isExcludedFromBulletinAbsence(fact.tag);
  });
}

export function factsByEleveId(facts: OccupancyFact[]): Map<string, OccupancyFact> {
  const map = new Map<string, OccupancyFact>();
  for (const f of facts) map.set(f.eleveId, f);
  return map;
}
