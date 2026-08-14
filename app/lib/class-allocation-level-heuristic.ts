import type { ClassLevel } from "@/app/lib/class-allocation-types";

function norm(classe: string): string {
  return classe
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

/** Détection approximative du niveau à partir du libellé de classe (liste élèves). */
function guessClassLevelFromClasse(classe: string | undefined): ClassLevel | null {
  const c = norm(String(classe || ""));
  if (!c) return null;

  if (/^(ps|ms|gs|cp|ce1|ce2|cm1|cm2)\b/.test(c)) return "ecole";
  if (/^(6e|5e|4e|3e|6eme|5eme|4eme|3eme)\b/.test(c)) return "college";
  if (/^(2nde|seconde|1ere|1re|terminale|tle)\b/.test(c)) return "lycee";

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
