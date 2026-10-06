/** Cache sessionStorage fiche dossier élève — scopé utilisateur (RGPD / multi-compte). */

const KEY_PREFIX = "scola:eleve-dossier:";

export function eleveDossierSessionCacheKey(userId: string, eleveId: string): string {
  const uid = userId.trim();
  if (!uid) {
    throw new Error("eleveDossierSessionCacheKey: user id must be loaded");
  }
  return `${KEY_PREFIX}${uid}:${eleveId}`;
}

export function clearEleveDossierSessionCaches(): void {
  if (typeof sessionStorage === "undefined") return;
  for (let i = sessionStorage.length - 1; i >= 0; i--) {
    const key = sessionStorage.key(i);
    if (key?.startsWith(KEY_PREFIX)) {
      sessionStorage.removeItem(key);
    }
  }
}
