import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import {
  inferPoleFromClassName,
  loadOfficialSchoolClasses,
  RECTORAT_LOCKED_POLES,
  resolveCanonicalSiecleClass,
} from "@/app/lib/nomenclature-classes";

const LOCKED_POLES = new Set<string>(RECTORAT_LOCKED_POLES);

/**
 * Pour collège/lycée : aligne la classe élève sur le CODE_STRUCTURE rectorat (import Excel/Siècle).
 * École : laissée telle quelle (pas de matching rectorat pour l'instant).
 */
export async function normalizeElevesToSiecleClasses(
  etablissementId: string,
  eleves: EleveConfig[],
): Promise<{ eleves: EleveConfig[]; normalized: number; unresolved: string[] }> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  if (!official.hasLockedSiecle) {
    return { eleves, normalized: 0, unresolved: [] };
  }

  let normalized = 0;
  const unresolvedSet = new Set<string>();

  const out = eleves.map((e) => {
    const raw = String(e.classe || "").trim();
    if (!raw) return e;

    const pole = inferPoleFromClassName(raw);
    if (!LOCKED_POLES.has(pole)) return e;

    const canonical = resolveCanonicalSiecleClass(
      raw,
      official.canonicalByFold,
      official.lockedClasses,
    );
    if (!canonical) {
      unresolvedSet.add(raw);
      return e;
    }
    if (canonical !== raw) normalized += 1;
    return { ...e, classe: canonical };
  });

  return {
    eleves: out,
    normalized,
    unresolved: [...unresolvedSet].sort((a, b) => a.localeCompare(b, "fr")),
  };
}
