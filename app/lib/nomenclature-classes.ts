import "server-only";

import { listNomenclatureByType, type NomenclatureEntry } from "@/app/lib/ref-nomenclature-db";
import { foldSchoolClass, schoolClassesMatch } from "@/app/lib/school-classes-catalog";

export type SiecleDivision = NomenclatureEntry;

export type OfficialClassesResult = {
  /** Siècle Structures.xml importé → seule source autorisée. */
  source: "siecle" | "fallback";
  classes: string[];
  classesByPole: Record<string, string[]>;
  divisions: SiecleDivision[];
  /** foldSchoolClass → code officiel Siècle (ex. 1A → 1 A). */
  canonicalByFold: Map<string, string>;
};

function inferPoleForDivision(code: string, libelle: string | null): string {
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

function buildCanonicalByFold(divisions: SiecleDivision[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const d of divisions) {
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

/** Codes division Siècle triés (CODE_STRUCTURE). */
export async function listSiecleClasses(etablissementId: string): Promise<string[]> {
  const divisions = await listSiecleDivisions(etablissementId);
  return divisions
    .map((d) => d.code)
    .sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

/**
 * Charge les classes officielles.
 * Si Structures.xml a été importé : **uniquement** les divisions Siècle (pas de classes manuelles).
 */
export async function loadOfficialSchoolClasses(
  etablissementId: string,
): Promise<OfficialClassesResult> {
  const divisions = await listSiecleDivisions(etablissementId);
  const canonicalByFold = buildCanonicalByFold(divisions);

  if (divisions.length === 0) {
    return {
      source: "fallback",
      classes: [],
      classesByPole: {},
      divisions: [],
      canonicalByFold,
    };
  }

  const classes = divisions
    .map((d) => d.code)
    .sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));

  return {
    source: "siecle",
    classes,
    classesByPole: groupDivisionsByPole(divisions),
    divisions,
    canonicalByFold,
  };
}

/** Résout une classe saisie (1A, 1 A, 1-A…) vers le CODE_STRUCTURE Siècle. */
export function resolveCanonicalSiecleClass(
  raw: string | null | undefined,
  canonicalByFold: Map<string, string>,
  officialClasses: string[],
): string | null {
  const trimmed = String(raw || "").trim();
  if (!trimmed) return null;

  if (officialClasses.includes(trimmed)) return trimmed;

  const fold = foldSchoolClass(trimmed);
  if (!fold) return null;

  const fromMap = canonicalByFold.get(fold);
  if (fromMap) return fromMap;

  for (const official of officialClasses) {
    if (schoolClassesMatch(trimmed, official)) return official;
  }

  return null;
}

export async function resolveCanonicalSiecleClassForEtab(
  etablissementId: string,
  raw: string | null | undefined,
): Promise<string | null> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  if (official.source !== "siecle") return String(raw || "").trim() || null;
  return resolveCanonicalSiecleClass(raw, official.canonicalByFold, official.classes);
}

/** Élèves dont la classe ne correspond à aucune division Siècle (après normalisation fold). */
export async function listUnmatchedEleveClasses(
  etablissementId: string,
  eleveClasses: string[],
): Promise<string[]> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  if (official.source !== "siecle") return [];

  const unmatched = new Set<string>();
  for (const raw of eleveClasses) {
    const t = String(raw || "").trim();
    if (!t) continue;
    if (!resolveCanonicalSiecleClass(t, official.canonicalByFold, official.classes)) {
      unmatched.add(t);
    }
  }
  return [...unmatched].sort((a, b) => a.localeCompare(b, "fr"));
}

/** @deprecated Préférer loadOfficialSchoolClasses — ne fusionne plus les sources manuelles si Siècle présent. */
export async function mergeClassesWithSiecle(
  etablissementId: string,
  ...otherSources: string[][]
): Promise<string[]> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  if (official.source === "siecle") return official.classes;

  const seen = new Set<string>();
  const out: string[] = [];
  const push = (raw: string) => {
    const t = raw.trim();
    if (!t) return;
    const key = foldSchoolClass(t);
    if (seen.has(key)) return;
    seen.add(key);
    out.push(t);
  };
  for (const src of otherSources) {
    for (const c of src) push(c);
  }
  return out.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

export async function isKnownSiecleClass(
  etablissementId: string,
  classe: string | null | undefined,
): Promise<boolean> {
  const official = await loadOfficialSchoolClasses(etablissementId);
  if (official.source !== "siecle") return false;
  return (
    resolveCanonicalSiecleClass(classe, official.canonicalByFold, official.classes) != null
  );
}
