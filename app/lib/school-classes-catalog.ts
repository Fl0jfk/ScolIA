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

/** Compacte un libellé de classe pour comparaison (PS A ≡ PSA, 6ème A ≡ 6A, CE1-B ≡ CE1B). */
export function foldSchoolClass(raw: string): string {
  let s = raw
    .trim()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[°º]/g, "")
    .replace(/[()[\]]/g, " ")
    .toUpperCase();

  // Niveaux écrits en toutes lettres / ordinals Pronote → formes compactes.
  s = s
    .replace(/\bPREMIERE\b/g, "1RE")
    .replace(/\b1ERE\b/g, "1RE")
    .replace(/\bSECONDE\b/g, "2NDE")
    .replace(/\b2DE\b/g, "2NDE")
    .replace(/\bTERMINALE\b/g, "TLE")
    .replace(/\bTALE\b/g, "TLE")
    .replace(/\b([3-6])EME\b/g, "$1E")
    .replace(/\b([3-6])E\b/g, "$1E");

  s = s.replace(/[\s._\-/]+/g, "");

  // 6EA (depuis 6ème A) → 6A ; 1REA → 1A ; 2NDEA → 2A ; TLEA → TA
  s = s.replace(/^([3-6])E([A-Z0-9]+)$/, "$1$2");
  s = s.replace(/^1RE([A-Z0-9]+)$/, "1$1");
  s = s.replace(/^2NDE([A-Z0-9]+)$/, "2$1");
  s = s.replace(/^TLE([A-Z0-9]+)$/, "T$1");

  return s;
}

export function schoolClassesMatch(
  a: string | null | undefined,
  b: string | null | undefined,
): boolean {
  const fa = foldSchoolClass(String(a || ""));
  const fb = foldSchoolClass(String(b || ""));
  if (!fa || !fb) return false;
  if (fa === fb) return true;
  // Préfixe prudent (évite « 1 » ≡ « 1A ») : au moins 3 caractères utiles.
  if (fa.length >= 3 && fb.length >= 3 && (fa.startsWith(fb) || fb.startsWith(fa))) {
    return true;
  }
  return false;
}

function foldClass(raw: string): string {
  return foldSchoolClass(raw);
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

/**
 * Catalogue réservation de salles : enrichissement école + conservation du pôle MAINTENANCE.
 * (Les séjours / dossiers excluent toujours MAINTENANCE via resolveClassesByPoleCatalog.)
 */
export function resolveProfRoomClassesByPole(
  ...sources: Array<Record<string, string[]> | null | undefined>
): Record<string, string[]> {
  const rawMerged: Record<string, string[]> = {};
  for (const src of sources) {
    if (!src) continue;
    for (const [pole, list] of Object.entries(src)) {
      const cur = rawMerged[pole] || [];
      const next = [...cur];
      for (const c of list || []) {
        const t = String(c || "").trim();
        if (t && !next.includes(t)) next.push(t);
      }
      rawMerged[pole] = next;
    }
  }

  const maintenanceKey = Object.keys(rawMerged).find((k) => k.toUpperCase() === "MAINTENANCE");
  const maintenanceClasses = maintenanceKey
    ? (rawMerged[maintenanceKey] || []).map((c) => String(c).trim()).filter(Boolean)
    : [];

  const enriched = resolveClassesByPoleCatalog(rawMerged);
  if (maintenanceClasses.length > 0) {
    enriched.MAINTENANCE = maintenanceClasses;
  } else {
    enriched.MAINTENANCE = ["MAINTENANCE"];
  }
  return enriched;
}
