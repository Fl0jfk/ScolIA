/**
 * Normalisation régime scolaire (Siècle CODE_REGIME / Excel Charlemagne).
 *
 * Source officielle — nomenclature BCN (Base Centrale de Nomenclatures),
 * documentée par l’API Particulier (Éducation nationale) :
 * https://particulier.api.gouv.fr/catalogue/education_nationale/statut_eleve_scolarise_v4
 *
 * | Code | Libellé BCN |
 * | 0 | Externe libre |
 * | 1 | Externe surveillé |
 * | 2 | Demi-pensionnaire dans l’établissement |
 * | 3 | Interne dans l’établissement |
 * | 4 | Interne externé (inscrit internat, ne dort pas sur place) |
 * | 5 | Interne hébergé (dort dans un autre établissement) |
 * | 6 | Demi-pensionnaire hors établissement |
 *
 * Pour l’appel de nuit / module internat : seuls les codes qui dorment
 * dans l’établissement (3) — éventuellement 5 si on gère l’hébergement croisé.
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

  // Codes numériques BCN / Siècle BEE (API Particulier)
  if (s === "3" || s === "5") return "interne";
  if (s === "2" || s === "6") return "demi_pension";
  if (s === "0" || s === "1" || s === "4") return "externe";
  // 4 = interne externé → ne dort pas : traité comme externe pour l'appel nuit

  if (/\binterne[- ]?externe\b/.test(s) || s === "ie" || s.includes("int ext")) {
    // « Interne externé » BCN : pas de nuit sur place
    return "externe";
  }
  if (
    /\binterne\b/.test(s) ||
    s === "i" ||
    s === "int" ||
    s.startsWith("int ") ||
    s.includes("internat")
  ) {
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

  // Booléens de colonne « Interne » (oui/non) — jamais les chiffres 0–6 (codes BCN).
  if (s === "oui" || s === "o" || s === "yes" || s === "true" || s === "x") return "interne";
  if (s === "non" || s === "no" || s === "false") return "externe";

  return "inconnu";
}

/**
 * Libellé canonique pour stockage (référentiel / Excel).
 * Codes BCN et synonymes → Interne | Demi-pension | Externe.
 */
export function canonicalRegimeLabel(raw: string | undefined | null): string | undefined {
  const trimmed = String(raw ?? "").trim();
  if (!trimmed) return undefined;
  switch (classifyRegime(trimmed)) {
    case "interne":
      return "Interne";
    case "demi_pension":
      return "Demi-pension";
    case "externe":
      return "Externe";
    case "inconnu":
      return trimmed;
  }
}
