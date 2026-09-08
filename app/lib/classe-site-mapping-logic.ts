import { foldSchoolClass, schoolClassesMatch } from "@/app/lib/school-classes-catalog";
import { guessClassLevelFromClasse } from "@/app/lib/class-allocation-level-heuristic";
import type { ClassLevel } from "@/app/lib/class-allocation-types";

export type ClasseSiteMappingRecord = {
  classKey: string;
  className: string;
  siteId: string;
  siecleCode: string | null;
};

export type SiecleDivisionOption = {
  code: string;
  libelle: string | null;
};

export function classeMappingKey(className: string): string {
  return foldSchoolClass(className);
}

export function suggestSiecleCode(
  className: string,
  divisions: SiecleDivisionOption[],
): string | null {
  const cls = className.trim();
  if (!cls || divisions.length === 0) return null;
  const folded = foldSchoolClass(cls);
  if (!folded) return null;

  for (const d of divisions) {
    if (foldSchoolClass(d.code) === folded) return d.code;
  }
  for (const d of divisions) {
    const lib = String(d.libelle || "").trim();
    if (lib && foldSchoolClass(lib) === folded) return d.code;
  }
  for (const d of divisions) {
    if (schoolClassesMatch(d.code, cls)) return d.code;
    if (d.libelle && schoolClassesMatch(d.libelle, cls)) return d.code;
  }
  return null;
}

export function resolveMappedSiteId(
  className: string | null | undefined,
  mappings: ClasseSiteMappingRecord[],
): string | null {
  const cls = String(className || "").trim();
  if (!cls || mappings.length === 0) return null;
  const key = foldSchoolClass(cls);
  if (!key) return null;

  for (const m of mappings) {
    if (m.classKey === key || foldSchoolClass(m.className) === key) return m.siteId;
  }
  for (const m of mappings) {
    const siecle = String(m.siecleCode || "").trim();
    if (!siecle) continue;
    if (foldSchoolClass(siecle) === key || schoolClassesMatch(siecle, cls)) return m.siteId;
  }
  return null;
}

export function guessSiteKindForClass(className: string): ClassLevel | null {
  return guessClassLevelFromClasse(className);
}
