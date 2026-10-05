import { schoolClassesMatch } from "@/app/lib/school-classes-catalog";

/** Valeur technique pour l’option « Autres » (saisie libre). */
export const TRAVELS_CLASSES_AUTRES_VALUE = "__AUTRES__";
export const TRAVELS_CLASSES_AUTRES_LABEL = "Autres";

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
  const selected: string[] = [];
  const otherParts: string[] = [];
  for (const t of tokens) {
    if (t === TRAVELS_CLASSES_AUTRES_LABEL || t === TRAVELS_CLASSES_AUTRES_VALUE) {
      continue;
    }
    const hit =
      catalog.find((c) => c.toLowerCase() === t.toLowerCase()) ||
      catalog.find((c) => schoolClassesMatch(c, t)) ||
      null;
    if (hit) {
      if (!selected.includes(hit)) selected.push(hit);
    } else {
      otherParts.push(t);
    }
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

/**
 * Ordonne les classes disponibles pour l’onglet Élèves :
 * d’abord celles déjà choisies à la création du séjour, puis le reste (alpha).
 * Ne rajoute aucune classe hors catalogue (les fantômes N-1 restent exclus).
 */
export function prioritizeClassesForTrip(
  availableClasses: string[],
  tripClassesRaw: string | null | undefined,
): string[] {
  const tripTokens = splitClassesValue(String(tripClassesRaw || ""));
  if (tripTokens.length === 0) {
    return [...availableClasses].sort((a, b) =>
      a.localeCompare(b, "fr", { sensitivity: "base", numeric: true }),
    );
  }

  const priority: string[] = [];
  const rest: string[] = [];
  const seen = new Set<string>();

  for (const cls of availableClasses) {
    const key = cls.trim();
    if (!key || seen.has(key)) continue;
    seen.add(key);
    const isTripClass = tripTokens.some((t) => schoolClassesMatch(cls, t));
    if (isTripClass) priority.push(cls);
    else rest.push(cls);
  }

  const byLabel = (a: string, b: string) =>
    a.localeCompare(b, "fr", { sensitivity: "base", numeric: true });
  priority.sort(byLabel);
  rest.sort(byLabel);
  return [...priority, ...rest];
}

/** Indique si une classe fait partie de la sélection initiale du séjour. */
export function isTripSelectedClass(
  classe: string,
  tripClassesRaw: string | null | undefined,
): boolean {
  const tripTokens = splitClassesValue(String(tripClassesRaw || ""));
  if (!tripTokens.length) return false;
  return tripTokens.some((t) => schoolClassesMatch(classe, t));
}
