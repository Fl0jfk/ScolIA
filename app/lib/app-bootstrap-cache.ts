import type { AppContextPayload, SitePublicIdentity } from "@/app/contexts/admin-bootstrap";

const STORAGE_KEY_PREFIX = "scola.bootstrap:v1:";
/** Données stables (logo, accent, modules) — revalidation en arrière-plan. */
const MAX_AGE_MS = 30 * 60 * 1000;

type BootstrapCachePayload = {
  sitePublic: SitePublicIdentity;
  appContext: AppContextPayload | null;
  savedAt: number;
};

function storageKey(): string | null {
  if (typeof window === "undefined") return null;
  return `${STORAGE_KEY_PREFIX}${window.location.hostname}`;
}

export function readBootstrapCache(): BootstrapCachePayload | null {
  const key = storageKey();
  if (!key) return null;
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as BootstrapCachePayload;
    if (!parsed?.savedAt || Date.now() - parsed.savedAt > MAX_AGE_MS) {
      localStorage.removeItem(key);
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

export function writeBootstrapCache(payload: Omit<BootstrapCachePayload, "savedAt">): void {
  const key = storageKey();
  if (!key) return;
  try {
    const next: BootstrapCachePayload = { ...payload, savedAt: Date.now() };
    localStorage.setItem(key, JSON.stringify(next));
  } catch {
    /* quota / mode privé */
  }
}

export function clearBootstrapCache(): void {
  const key = storageKey();
  if (!key) return;
  try {
    localStorage.removeItem(key);
  } catch {
    /* ignore */
  }
}
