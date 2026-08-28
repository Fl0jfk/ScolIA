import "server-only";

import { listDivisionCodes } from "@/app/lib/ref-nomenclature-db";
import { foldSchoolClass } from "@/app/lib/school-classes-catalog";

/** Classes Siècle (Structures.xml → type division) — source de vérité si importée. */
export async function listSiecleClasses(etablissementId: string): Promise<string[]> {
  const codes = await listDivisionCodes(etablissementId);
  return [...codes].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

/** Fusionne classes Siècle avec d'autres sources (élèves, stages…) sans doublons. */
export async function mergeClassesWithSiecle(
  etablissementId: string,
  ...otherSources: string[][]
): Promise<string[]> {
  const siecle = await listSiecleClasses(etablissementId);
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

  for (const c of siecle) push(c);
  for (const src of otherSources) {
    for (const c of src) push(c);
  }

  return out.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
}

/** Vérifie qu'une classe élève correspond à une division Siècle importée. */
export async function isKnownSiecleClass(
  etablissementId: string,
  classe: string | null | undefined,
): Promise<boolean> {
  const raw = String(classe || "").trim();
  if (!raw) return false;
  const divisions = await listSiecleClasses(etablissementId);
  const key = foldSchoolClass(raw);
  return divisions.some((d) => foldSchoolClass(d) === key);
}
