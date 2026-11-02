/** Génération invalidation bundles signaux — repli mémoire (tests / sans Valkey). */

const generationByEtab = new Map<string, number>();

function normalizeEtabId(etablissementId: string): string {
  const id = etablissementId.trim();
  return id || "default";
}

export function getDashboardSignalsBundleGenerationMemory(etablissementId: string): number {
  return generationByEtab.get(normalizeEtabId(etablissementId)) ?? 0;
}

export function bumpDashboardSignalsBundleGenerationMemory(etablissementId: string): number {
  const etab = normalizeEtabId(etablissementId);
  const next = (generationByEtab.get(etab) ?? 0) + 1;
  generationByEtab.set(etab, next);
  return next;
}

export function resetDashboardSignalsBundleGenerationForTests(): void {
  generationByEtab.clear();
}
