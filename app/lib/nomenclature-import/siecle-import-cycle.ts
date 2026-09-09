/**
 * Siècle exporte collège et lycée séparément (UAJ / Structures / Élèves distincts).
 * Le cycle choisi à l'import permet d'empiler les deux jeux sans écraser l'autre.
 */
export const SIECLE_IMPORT_CYCLES = ["college", "lycee"] as const;

export type SiecleImportCycle = (typeof SIECLE_IMPORT_CYCLES)[number];

/** Fichiers propres à un UAJ / un cycle (à importer une fois par collège et une fois par lycée). */
export const SIECLE_CYCLE_SCOPED_KINDS = [
  "communs",
  "structures",
  "eleves",
  "responsables",
] as const;

/** Fichiers référentiels partagés (un seul import suffit pour tout l'établissement). */
export const SIECLE_SHARED_KINDS = [
  "nomenclature",
  "geographique",
  "etablissements",
] as const;

export type SiecleCycleScopedKind = (typeof SIECLE_CYCLE_SCOPED_KINDS)[number];

export function isSiecleImportCycle(value: unknown): value is SiecleImportCycle {
  return value === "college" || value === "lycee";
}

export function parseSiecleImportCycle(raw: unknown): SiecleImportCycle | null {
  if (typeof raw !== "string") return null;
  const v = raw.trim().toLowerCase();
  if (v === "college" || v === "collège") return "college";
  if (v === "lycee" || v === "lycée") return "lycee";
  return null;
}

export function isSiecleCycleScopedKind(kind: string): kind is SiecleCycleScopedKind {
  return (SIECLE_CYCLE_SCOPED_KINDS as readonly string[]).includes(kind);
}

export function siecleCyclePole(cycle: SiecleImportCycle): "COLLÈGE" | "LYCÉE" {
  return cycle === "college" ? "COLLÈGE" : "LYCÉE";
}

export function siecleCycleLabel(cycle: SiecleImportCycle): string {
  return cycle === "college" ? "Collège" : "Lycée";
}
