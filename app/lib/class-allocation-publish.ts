import "server-only";

import type { ClassAllocationRun } from "@/app/lib/class-allocation-types";
import { loadElevesRegistry, saveElevesRegistry } from "@/app/lib/eleves-registry";

export type PublishClassAllocationResult = {
  updated: number;
  unchanged: number;
  missingInes: string[];
  classesTouched: string[];
};

/**
 * Applique un run de répartition au registre élèves (classe cible par INE).
 * Met aussi à jour la scolarité courante via saveElevesRegistry → upsert DB.
 */
export async function publishClassAllocationRun(
  run: ClassAllocationRun,
): Promise<PublishClassAllocationResult> {
  const assignments = new Map<string, string>();
  const classesTouched = new Set<string>();

  for (const entries of Object.values(run.levelResults)) {
    for (const entry of entries) {
      const className = entry.className.trim();
      if (!className) continue;
      classesTouched.add(className);
      for (const rawIne of entry.studentInes) {
        const ine = String(rawIne || "")
          .trim()
          .toUpperCase();
        if (!ine) continue;
        assignments.set(ine, className);
      }
    }
  }

  if (assignments.size === 0) {
    return { updated: 0, unchanged: 0, missingInes: [], classesTouched: [] };
  }

  const eleves = await loadElevesRegistry();
  const missingInes: string[] = [];
  const seen = new Set<string>();
  let updated = 0;
  let unchanged = 0;

  const next = eleves.map((e) => {
    const ine = String(e.ine || "")
      .trim()
      .toUpperCase();
    if (!ine || !assignments.has(ine)) return e;
    seen.add(ine);
    const target = assignments.get(ine)!;
    if ((e.classe || "").trim() === target) {
      unchanged += 1;
      return e;
    }
    updated += 1;
    return { ...e, classe: target };
  });

  for (const ine of assignments.keys()) {
    if (!seen.has(ine)) missingInes.push(ine);
  }

  if (updated > 0) {
    await saveElevesRegistry(next);
  }

  return {
    updated,
    unchanged,
    missingInes: missingInes.slice(0, 50),
    classesTouched: [...classesTouched].sort((a, b) => a.localeCompare(b, "fr")),
  };
}
