import "server-only";

import type { EleveConfig } from "@/app/lib/eleves-config";
import {
  loadOfficialSchoolClasses,
  resolveCanonicalSiecleClass,
} from "@/app/lib/nomenclature-classes";

/** Normalise les classes élèves vers le CODE_STRUCTURE Siècle quand Structures.xml est importé. */
export async function normalizeElevesToSiecleClasses(
  etablissementId: string,
  eleves: EleveConfig[],
): Promise<{ eleves: EleveConfig[]; normalized: number; unresolved: string[] }> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  if (official.source !== "siecle") {
    return { eleves, normalized: 0, unresolved: [] };
  }

  let normalized = 0;
  const unresolvedSet = new Set<string>();

  const out = eleves.map((e) => {
    const raw = String(e.classe || "").trim();
    if (!raw) return e;

    const canonical = resolveCanonicalSiecleClass(
      raw,
      official.canonicalByFold,
      official.classes,
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
