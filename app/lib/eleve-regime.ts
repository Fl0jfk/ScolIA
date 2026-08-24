/**
 * Normalisation régime scolaire (Siècle CODE_REGIME / Excel Charlemagne).
 * Codes Siècle courants : 0 externe, 1 DP, 2 interne, 3 interne-externe.
 */

export type EleveRegimeKind = "interne" | "demi_pension" | "externe" | "inconnu";

export function normalizeRegimeLabel(raw: string): string {
  return String(raw ?? "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** True si l'élève dort à l'internat (appel surveillants). */
export function isRegimeInterne(raw: string | undefined | null): boolean {
  return classifyRegime(raw) === "interne";
}

export function classifyRegime(raw: string | undefined | null): EleveRegimeKind {
  const s = normalizeRegimeLabel(String(raw ?? ""));
  if (!s) return "inconnu";

  // Codes numériques Siècle / STS
  if (s === "2" || s === "3") return "interne";
  if (s === "1") return "demi_pension";
  if (s === "0") return "externe";

  if (/\binterne[- ]?externe\b/.test(s) || s === "ie" || s.includes("int ext")) {
    return "interne";
  }
  if (
    /\binterne\b/.test(s) ||
    s === "i" ||
    s === "int" ||
    s.startsWith("int ") ||
    s.includes("internat")
  ) {
    // « externe » contient parfois « externe » seul — déjà exclu ci-dessus pour interne-externe
    if (/\bexterne\b/.test(s) && !/\binterne\b/.test(s)) return "externe";
    return "interne";
  }
  if (
    s.includes("demi pension") ||
    s.includes("demipension") ||
    s === "dp" ||
    s.includes("1/2 pension") ||
    s.includes("half board")
  ) {
    return "demi_pension";
  }
  if (s.includes("externe") || s === "ext" || s === "e") return "externe";

  return "inconnu";
}
