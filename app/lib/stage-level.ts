/**
 * Déduit un niveau scolaire à partir du libellé de classe
 * (ex. « 3e2 » → 3e, « 2A » → 2nde). Utilisable côté client et serveur.
 */
export function inferStudentLevelFromClass(className: string): string {
  const raw = className.trim().toLowerCase();
  if (!raw) return "3e";
  const c = raw
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[\s._-]+/g, "");

  // Collège
  if (/^(6e|6eme)/.test(c) || /^6[a-z0-9]/.test(c)) return "6e";
  if (/^(5e|5eme)/.test(c) || /^5[a-z0-9]/.test(c)) return "5e";
  if (/^(4e|4eme)/.test(c) || /^4[a-z0-9]/.test(c)) return "4e";
  if (/^(3e|3eme)/.test(c) || /^3[a-z0-9]/.test(c)) return "3e";

  // Lycée — « 2A », « 2B », « 2nde1 »… = seconde (pas le fallback 3e)
  if (/^(2nde|2de|seconde)/.test(c) || /^2[a-z0-9]/.test(c) || c.includes("2nde") || c.includes("seconde")) {
    return "2nde";
  }
  if (
    /^(1re|1ere|premiere)/.test(c) ||
    /^1[a-z]/.test(c) ||
    /^1st/.test(c) ||
    c.includes("1re") ||
    c.includes("prem")
  ) {
    return "1re";
  }
  if (
    /^(tle|terminale|tale)/.test(c) ||
    /^t[a-z0-9]/.test(c) ||
    c.includes("tle") ||
    c.includes("term")
  ) {
    return "Tle";
  }

  return "3e";
}

/** Niveaux collège / lycée pour les stages (ordre d'affichage). */
export const STAGE_LEVEL_OPTIONS = ["6e", "5e", "4e", "3e", "2nde", "1re", "Tle"] as const;

export type StageLevelOption = (typeof STAGE_LEVEL_OPTIONS)[number];
