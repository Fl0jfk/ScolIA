import { resolveClassesByPoleCatalog } from "@/app/lib/school-classes-catalog";

/** Valeur technique pour l’option « Autres » (saisie libre). */
export const TRAVELS_CLASSES_AUTRES_VALUE = "__AUTRES__";
export const TRAVELS_CLASSES_AUTRES_LABEL = "Autres";

function flattenClassesByPole(classesByPole?: Record<string, string[]> | null): string[] {
  if (!classesByPole || typeof classesByPole !== "object") return [];
  const out: string[] = [];
  const seen = new Set<string>();
  for (const list of Object.values(classesByPole)) {
    for (const raw of list || []) {
      const c = String(raw || "").trim();
      if (!c || seen.has(c)) continue;
      seen.add(c);
      out.push(c);
    }
  }
  return out.sort((a, b) => a.localeCompare(b, "fr"));
}

/** Fusionne les catalogues salles + enseignements transversaux (sans MAINTENANCE, école enrichie). */
export function mergeTripClassCatalogs(
  ...sources: Array<Record<string, string[]> | null | undefined>
): string[] {
  return flattenClassesByPole(resolveClassesByPoleCatalog(...sources));
}

export function splitClassesValue(raw: string): string[] {
  return String(raw || "")
    .split(/[,;/]+/)
    .map((c) => c.trim())
    .filter(Boolean);
}

function joinClassesValue(parts: string[]): string {
  return parts.map((c) => c.trim()).filter(Boolean).join(", ");
}

export function parseClassesSelection(
  raw: string,
  catalog: string[],
): { selected: string[]; otherText: string; autres: boolean } {
  const tokens = splitClassesValue(raw);
  const catalogSet = new Set(catalog.map((c) => c.toLowerCase()));
  const selected: string[] = [];
  const otherParts: string[] = [];
  for (const t of tokens) {
    if (t === TRAVELS_CLASSES_AUTRES_LABEL || t === TRAVELS_CLASSES_AUTRES_VALUE) {
      continue;
    }
    const hit = catalog.find((c) => c.toLowerCase() === t.toLowerCase());
    if (hit) selected.push(hit);
    else if (!catalogSet.has(t.toLowerCase())) otherParts.push(t);
  }
  const otherText = otherParts.join(", ");
  return { selected, otherText, autres: otherText.length > 0 };
}

export function serializeClassesSelection(selected: string[], otherText: string): string {
  const parts = [...selected];
  const extra = otherText.trim();
  if (extra) parts.push(extra);
  return joinClassesValue(parts);
}
