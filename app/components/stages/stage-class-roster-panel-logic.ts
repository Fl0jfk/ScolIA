export type RosterPanelResponse = {
  availableClasses: string[];
  roster: { className: string } | null;
};

export function rosterMemoryCacheKey(className: string): string {
  return className.trim().toLowerCase() || "__default__";
}

/** Classe à charger après une réponse « métadonnées seules » (sans roster). */
export function pickClassAfterMetadataResponse(
  availableClasses: string[],
  wanted: string,
  previousSelected: string,
): string | null {
  if (wanted.trim()) return null;
  if (!availableClasses.length) return null;
  const prev = previousSelected.trim();
  if (
    prev &&
    availableClasses.some((c) => c.localeCompare(prev, "fr", { sensitivity: "base" }) === 0)
  ) {
    return prev;
  }
  return availableClasses[0] ?? null;
}

/** M1 : ne pas écrire un roster null sous une classe (pollution du cache mémoire). */
export function rosterMemoryWriteKeys(
  json: RosterPanelResponse,
  wanted: string,
): string[] {
  if (!json.roster) return [];
  const className = json.roster.className || wanted.trim();
  if (!className) return [];
  const keys = [rosterMemoryCacheKey(className)];
  if (!wanted.trim()) keys.push("__default__");
  return keys;
}
