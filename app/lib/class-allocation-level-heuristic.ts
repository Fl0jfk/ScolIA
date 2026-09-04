import type { ClassLevel } from "@/app/lib/class-allocation-types";

/** Compacte un libellé pour détection de cycle (JE3 MME…, 5°B, CP MME…). */
function compactClasse(classe: string): string {
  return classe
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[°º]/g, "")
    .toLowerCase()
    .replace(/[()[\]]/g, " ")
    .replace(/[\s._\-/]+/g, "")
    .trim();
}

/** Détection approximative du niveau à partir du libellé de classe (liste élèves). */
export function guessClassLevelFromClasse(classe: string | undefined): ClassLevel | null {
  const raw = String(classe || "").trim();
  if (!raw) return null;

  const spaced = raw
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[°º]/g, " ")
    .toLowerCase()
    .replace(/[()[\]]/g, " ")
    .replace(/[\s._\-/]+/g, " ")
    .trim();
  const c = compactClasse(raw);
  if (!c) return null;

  // Jardin d’enfants / petite section Providence : « JE3 MME DOUGHTY », « JE4  MME LOURDEL »
  if (/^je[1-5]?\b/.test(spaced) || /^je[1-5]?/.test(c)) return "ecole";

  // Maternelle / élémentaire — y compris « CP MME PICHURON », « CPA », « CE1B »
  if (
    /^(tps|ps|ms|gs|cp|ce1|ce2|cm1|cm2)\b/.test(spaced) ||
    /^(tps|ps|ms|gs|cp|ce1|ce2|cm1|cm2)/.test(c) ||
    /\b(maternelle|elementaire|primaire|ecole)\b/.test(spaced)
  ) {
    return "ecole";
  }

  // Collège : « 6A », « 3B », « 5°B », « 6ème F », « 3 e A »
  if (
    /^(6e|5e|4e|3e|6eme|5eme|4eme|3eme)\b/.test(spaced) ||
    /^(6|5|4|3)(e|eme)?[a-z0-9]*$/.test(c) ||
    /^(6|5|4|3)\s*(e|eme)?\s*[a-z0-9]*$/.test(spaced)
  ) {
    return "college";
  }

  // Lycée : 2nde / 1re / Tle — avant le fallback « T… » pour ne pas prendre TPS
  if (
    /^(2nde|seconde|2de|1ere|1re|premiere|terminale|tle)\b/.test(spaced) ||
    /^(2nde|2de|1ere|1re|tle|terminale)/.test(c)
  ) {
    return "lycee";
  }

  // Codes courts type « 2A », « 1C », « TA »
  if (/^(2|1)[a-z0-9]*$/.test(c)) return "lycee";
  if (/^t[a-z0-9]{0,2}$/.test(c)) return "lycee";

  return null;
}

export function uniqueClassesByLevelFromEleves(
  eleves: { classe?: string }[],
): Record<ClassLevel, string[]> {
  const buckets: Record<ClassLevel, Set<string>> = {
    ecole: new Set(),
    college: new Set(),
    lycee: new Set(),
  };

  for (const e of eleves) {
    const classe = String(e.classe || "").trim();
    if (!classe) continue;
    const level = guessClassLevelFromClasse(classe);
    if (!level) continue;
    buckets[level].add(classe);
  }

  return {
    ecole: Array.from(buckets.ecole).sort((a, b) => a.localeCompare(b, "fr")),
    college: Array.from(buckets.college).sort((a, b) => a.localeCompare(b, "fr")),
    lycee: Array.from(buckets.lycee).sort((a, b) => a.localeCompare(b, "fr")),
  };
}
