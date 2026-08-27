/**
 * Catalogue de classes scolaire partagé (dossiers, séjours, résa salles, etc.).
 * École = maternelle + élémentaire avec divisions (comme le collège a 6A/6B…).
 */

import { sanitizeDomainPlanningClassesByPole } from "@/app/lib/domain-planning-defaults";

/** Maternelle (divisions courantes). */
export const DEFAULT_MATERNELLE_CLASSES = [
  "TPS",
  "PSA",
  "PSB",
  "MSA",
  "MSB",
  "GSA",
  "GSB",
] as const;

/** Élémentaire avec plusieurs classes par niveau. */
export const DEFAULT_ELEMENTAIRE_CLASSES = [
  "CPA",
  "CPB",
  "CPC",
  "CE1A",
  "CE1B",
  "CE1C",
  "CE2A",
  "CE2B",
  "CE2C",
  "CM1A",
  "CM1B",
  "CM1C",
  "CM2A",
  "CM2B",
  "CM2C",
] as const;

export const DEFAULT_ECOLE_CLASSES: string[] = [
  ...DEFAULT_MATERNELLE_CLASSES,
  ...DEFAULT_ELEMENTAIRE_CLASSES,
];

export const DEFAULT_CLASSES_BY_POLE: Record<string, string[]> = {
  ÉCOLE: [...DEFAULT_ECOLE_CLASSES],
  COLLÈGE: [
    "6A",
    "6B",
    "6C",
    "6D",
    "6E",
    "6F",
    "5A",
    "5B",
    "5C",
    "5D",
    "5E",
    "5F",
    "4A",
    "4B",
    "4C",
    "4D",
    "4E",
    "4F",
    "3A",
    "3B",
    "3C",
    "3D",
    "3E",
    "3F",
  ],
  LYCÉE: [
    "2A",
    "2B",
    "2C",
    "2D",
    "2E",
    "1A",
    "1B",
    "1C",
    "1D",
    "1E",
    "1F",
    "TA",
    "TB",
    "TC",
    "TD",
    "TE",
    "TF",
  ],
};

const BARE_ELEMENTAIRE = new Set(["CP", "CE1", "CE2", "CM1", "CM2"]);
const BARE_MATERNELLE = new Set(["TPS", "PS", "MS", "GS"]);

function foldClass(raw: string): string {
  return raw
    .trim()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[°º]/g, "")
    .replace(/[\s._\-/]+/g, "")
    .toUpperCase();
}

function isEcolePoleName(pole: string): boolean {
  const blob = pole
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[_\s-]+/g, "");
  return (
    blob.includes("ecole") ||
    blob.includes("primaire") ||
    blob.includes("elementaire") ||
    blob.includes("maternelle")
  );
}

function hasMaternelleClass(list: string[]): boolean {
  return list.some((c) => {
    const k = foldClass(c);
    return /^(TPS|PS|MS|GS)([A-Z0-9]*)?$/.test(k);
  });
}

/** Liste école « plate » type ancien défaut CP–CM2 (sans divisions ni maternelle complète). */
function isLegacyFlatEcoleList(list: string[]): boolean {
  if (list.length === 0) return true;
  const folds = list.map(foldClass).filter(Boolean);
  if (folds.length === 0) return true;
  const allBare = folds.every(
    (c) => BARE_ELEMENTAIRE.has(c) || BARE_MATERNELLE.has(c),
  );
  if (!allBare) return false;
  // Au moins une division (CPA, CE2B, PSA…) → catalogue déjà riche.
  const hasDivision = folds.some((c) => /^(TPS|PS|MS|GS|CP|CE1|CE2|CM1|CM2)[A-Z0-9]+$/.test(c));
  return !hasDivision;
}

/**
 * Normalise le catalogue école :
 * - retire MAINTENANCE
 * - remplace les listes CP–CM2 « plates » par maternelle + divisions A/B/C
 * - sinon injecte au moins la maternelle si absente
 */
export function enrichClassesByPoleForSchool(
  classesByPole: Record<string, string[]>,
): Record<string, string[]> {
  const cleaned = sanitizeDomainPlanningClassesByPole(classesByPole);
  const out: Record<string, string[]> = {};

  for (const [pole, rawList] of Object.entries(cleaned)) {
    const list = (rawList || [])
      .map((c) => String(c).trim())
      .filter((c) => c && foldClass(c) !== "MAINTENANCE");
    if (!isEcolePoleName(pole)) {
      out[pole] = list;
      continue;
    }
    if (isLegacyFlatEcoleList(list)) {
      out[pole] = [...DEFAULT_ECOLE_CLASSES];
      continue;
    }
    if (!hasMaternelleClass(list)) {
      const seen = new Set(list.map(foldClass));
      const injected = DEFAULT_MATERNELLE_CLASSES.filter((c) => !seen.has(foldClass(c)));
      out[pole] = [...injected, ...list];
      continue;
    }
    out[pole] = list;
  }

  return out;
}

/** Catalogue prêt à l’emploi (fallback + enrichissement). */
export function resolveClassesByPoleCatalog(
  ...sources: Array<Record<string, string[]> | null | undefined>
): Record<string, string[]> {
  const merged: Record<string, string[]> = {};
  for (const src of sources) {
    if (!src) continue;
    for (const [pole, list] of Object.entries(src)) {
      const cur = merged[pole] || [];
      const next = [...cur];
      for (const c of list || []) {
        const t = String(c || "").trim();
        if (t && !next.includes(t)) next.push(t);
      }
      merged[pole] = next;
    }
  }
  const enriched = enrichClassesByPoleForSchool(merged);
  if (Object.keys(enriched).length === 0) {
    return { ...DEFAULT_CLASSES_BY_POLE };
  }
  return enriched;
}
