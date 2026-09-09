/**
 * Siècle exporte collège et lycée séparément (UAJ / Structures / Nomenclature / Élèves…).
 * Chaque fichier s'importe une fois par cycle ; l'upsert conserve l'autre cycle.
 */
export const SIECLE_IMPORT_CYCLES = ["college", "lycee"] as const;

export type SiecleImportCycle = (typeof SIECLE_IMPORT_CYCLES)[number];

/** Tous les XML Siècle d'un export sont propres à un UAJ / un cycle. */
export const SIECLE_CYCLE_SCOPED_KINDS = [
  "communs",
  "nomenclature",
  "geographique",
  "etablissements",
  "structures",
  "eleves",
  "responsables",
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

/** Valeur colonne `ref_nomenclature.cycle` ('' = import historique non scindé). */
export function siecleCycleColumnValue(cycle: SiecleImportCycle | null | undefined): string {
  return cycle ?? "";
}
