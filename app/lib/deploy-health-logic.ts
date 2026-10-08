export type JournalEntry = { tag: string; when: number };

/**
 * Dernière migration du journal dont le hash (ou le tag legacy) est présent en BDD.
 * Ignore `created_at` (souvent = `when` du journal, pas l’ordre d’application réel).
 */
export function resolveLatestMigrationTag(
  journalEntries: JournalEntry[],
  tagToContentHash: Map<string, string>,
  appliedHashes: Iterable<string>,
): string | null {
  const applied = new Set(appliedHashes);
  let bestIdx = -1;
  let bestTag: string | null = null;

  for (let i = 0; i < journalEntries.length; i++) {
    const entry = journalEntries[i];
    const fileHash = tagToContentHash.get(entry.tag);
    const isApplied =
      (fileHash !== undefined && applied.has(fileHash)) || applied.has(entry.tag);
    if (isApplied && i > bestIdx) {
      bestIdx = i;
      bestTag = entry.tag;
    }
  }

  return bestTag;
}
