import "server-only";

import { listNomenclatureByType, type NomenclatureEntry } from "@/app/lib/ref-nomenclature-db";
import { foldSchoolClass, schoolClassesMatch } from "@/app/lib/school-classes-catalog";

export type SiecleDivision = NomenclatureEntry;

/** Pôles dont les classes sont imposées telles quelles par Structures.xml (rectorat). */
export const RECTORAT_LOCKED_POLES = ["COLLÈGE", "LYCÉE"] as const;

export type SchoolPole = "ÉCOLE" | "COLLÈGE" | "LYCÉE";

export type OfficialClassesResult = {
  /** Au moins une division collège/lycée importée depuis Siècle. */
  hasLockedSiecle: boolean;
  /** Divisions collège + lycée — codes exacts rectorat (CODE_STRUCTURE). */
  lockedClasses: string[];
  lockedClassesByPole: Partial<Record<SchoolPole, string[]>>;
  /** Toutes les divisions Siècle groupées (y compris école si présentes un jour). */
  classesByPole: Record<string, string[]>;
  divisions: SiecleDivision[];
  /** fold → code rectorat — uniquement collège/lycée (normalisation import Excel). */
  canonicalByFold: Map<string, string>;
};

function inferPoleForDivision(code: string, libelle: string | null): SchoolPole {
  const blob = `${code} ${libelle || ""}`
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toUpperCase();

  if (
    /\b(TPS|PS[^A-Z]|PSA|PSB|MSA|MSB|GSA|GSB|MATERNELLE)\b/.test(blob) ||
    /^(TPS|PS|MS|GS)/.test(foldSchoolClass(code))
  ) {
    return "ÉCOLE";
  }
  if (
    /\b(CP|CE1|CE2|CM1|CM2|PRIMAIRE|ELEMENTAIRE)\b/.test(blob) ||
    /^(CP|CE1|CE2|CM1|CM2)/.test(foldSchoolClass(code))
  ) {
    return "ÉCOLE";
  }
  if (
    /^[3456][\sA-Z]?/.test(code.trim()) ||
    /\b[3456](E|EME|ÈME)\b/.test(blob) ||
    /\b(COLLEGE|COLLÈGE)\b/.test(blob)
  ) {
    return "COLLÈGE";
  }
  return "LYCÉE";
}

/** Infère le pôle à partir d'un libellé de classe (élève, roster…). */
export function inferPoleFromClassName(className: string): SchoolPole {
  return inferPoleForDivision(className.trim(), className.trim());
}

function isLockedPole(pole: SchoolPole): boolean {
  return (RECTORAT_LOCKED_POLES as readonly string[]).includes(pole);
}

function buildCanonicalByFold(divisions: SiecleDivision[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const d of divisions) {
    const pole = inferPoleForDivision(d.code, d.libelleLong || d.libelleCourt);
    if (!isLockedPole(pole)) continue;
    const fold = foldSchoolClass(d.code);
    if (fold && !map.has(fold)) map.set(fold, d.code);
  }
  return map;
}

function groupDivisionsByPole(divisions: SiecleDivision[]): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  for (const d of divisions) {
    const pole = inferPoleForDivision(d.code, d.libelleLong || d.libelleCourt);
    const list = out[pole] || [];
    if (!list.includes(d.code)) list.push(d.code);
    out[pole] = list;
  }
  for (const pole of Object.keys(out)) {
    out[pole] = [...out[pole]!].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
  }
  return out;
}

/** Divisions Structures.xml — liste complète avec libellés. */
export async function listSiecleDivisions(etablissementId: string): Promise<SiecleDivision[]> {
  return listNomenclatureByType(etablissementId, "division");
}

/** Codes division collège + lycée (rectorat). */
export async function listSiecleLockedClasses(etablissementId: string): Promise<string[]> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  return official.lockedClasses;
}

/**
 * Charge les classes officielles Siècle.
 * Seuls collège et lycée sont imposés par le rectorat ; l'école reste hors périmètre pour l'instant.
 */
export async function loadOfficialSchoolClasses(
  etablissementId: string,
): Promise<OfficialClassesResult> {
  const divisions = await listSiecleDivisions(etablissementId);
  const classesByPole = groupDivisionsByPole(divisions);

  const lockedClassesByPole: Partial<Record<SchoolPole, string[]>> = {};
  const lockedClasses: string[] = [];

  for (const pole of RECTORAT_LOCKED_POLES) {
    const list = classesByPole[pole] || [];
    if (list.length) {
      lockedClassesByPole[pole] = list;
      lockedClasses.push(...list);
    }
  }

  lockedClasses.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));

  const lockedDivisions = divisions.filter((d) =>
    isLockedPole(inferPoleForDivision(d.code, d.libelleLong || d.libelleCourt)),
  );

  return {
    hasLockedSiecle: lockedClasses.length > 0,
    lockedClasses,
    lockedClassesByPole,
    classesByPole,
    divisions,
    canonicalByFold: buildCanonicalByFold(lockedDivisions),
  };
}

/** Résout une classe vers le CODE_STRUCTURE rectorat (collège/lycée uniquement). */
export function resolveCanonicalSiecleClass(
  raw: string | null | undefined,
  canonicalByFold: Map<string, string>,
  lockedClasses: string[],
): string | null {
  const trimmed = String(raw || "").trim();
  if (!trimmed) return null;

  const pole = inferPoleFromClassName(trimmed);
  if (!isLockedPole(pole)) return null;

  if (lockedClasses.includes(trimmed)) return trimmed;

  const fold = foldSchoolClass(trimmed);
  if (!fold) return null;

  const fromMap = canonicalByFold.get(fold);
  if (fromMap) return fromMap;

  for (const official of lockedClasses) {
    if (schoolClassesMatch(trimmed, official)) return official;
  }

  return null;
}

export async function resolveCanonicalSiecleClassForEtab(
  etablissementId: string,
  raw: string | null | undefined,
): Promise<string | null> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  if (!official.hasLockedSiecle) return String(raw || "").trim() || null;

  const pole = inferPoleFromClassName(String(raw || ""));
  if (!isLockedPole(pole)) return String(raw || "").trim() || null;

  return (
    resolveCanonicalSiecleClass(raw, official.canonicalByFold, official.lockedClasses) ||
    String(raw || "").trim() ||
    null
  );
}

/** Classes élèves collège/lycée absentes du référentiel Siècle. */
export async function listUnmatchedEleveClasses(
  etablissementId: string,
  eleveClasses: string[],
): Promise<string[]> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  if (!official.hasLockedSiecle) return [];

  const unmatched = new Set<string>();
  for (const raw of eleveClasses) {
    const t = String(raw || "").trim();
    if (!t) continue;
    const pole = inferPoleFromClassName(t);
    if (!isLockedPole(pole)) continue;
    if (!resolveCanonicalSiecleClass(t, official.canonicalByFold, official.lockedClasses)) {
      unmatched.add(t);
    }
  }
  return [...unmatched].sort((a, b) => a.localeCompare(b, "fr"));
}

/**
 * Fusionne la liste roster : rectorat (collège/lycée) + sources libres pour l'école.
 * Les classes collège/lycée saisies ailleurs sont ignorées si Siècle est importé.
 */
export function mergeOfficialAndLocalClasses(
  official: OfficialClassesResult,
  ...localSources: string[][]
): string[] {
  const seen = new Set<string>();
  const out: string[] = [];

  const push = (raw: string, allowWhenLocked: boolean) => {
    const t = raw.trim();
    if (!t) return;
    const pole = inferPoleFromClassName(t);
    if (official.hasLockedSiecle && isLockedPole(pole) && !allowWhenLocked) return;
    const key = foldSchoolClass(t);
    if (seen.has(key)) return;
    seen.add(key);
    out.push(t);
  };

  if (official.hasLockedSiecle) {
    for (const c of official.lockedClasses) push(c, true);
  }

  for (const src of localSources) {
    for (const c of src) push(c, false);
  }

  return out.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

/** Fusionne classesByPole : collège/lycée Siècle + école depuis config/catalogue local. */
export function mergeClassesByPoleWithSiecle(
  official: OfficialClassesResult,
  localByPole: Record<string, string[]>,
): Record<string, string[]> {
  const out: Record<string, string[]> = { ...localByPole };

  if (!official.hasLockedSiecle) return out;

  for (const pole of RECTORAT_LOCKED_POLES) {
    const siecleList = official.lockedClassesByPole[pole];
    if (siecleList?.length) out[pole] = [...siecleList];
  }

  return out;
}

export async function isKnownSiecleClass(
  etablissementId: string,
  classe: string | null | undefined,
): Promise<boolean> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  if (!official.hasLockedSiecle) return false;
  return (
    resolveCanonicalSiecleClass(classe, official.canonicalByFold, official.lockedClasses) != null
  );
}
