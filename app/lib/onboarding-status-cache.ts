const STORAGE_KEY_PREFIX = "scola.onboarding-status:v1:";
const MAX_AGE_MS = 10 * 60 * 1000;

type OnboardingStatusCache = {
  completed: boolean;
  step: number;
  isOrgAdmin: boolean;
  isPlatformMaster: boolean;
  savedAt: number;
};

function storageKey(): string | null {
  if (typeof window === "undefined") return null;
  return `${STORAGE_KEY_PREFIX}${window.location.hostname}`;
}

export function readOnboardingStatusCache(): Omit<OnboardingStatusCache, "savedAt"> | null {
  const key = storageKey();
  if (!key) return null;
  try {
    const raw = sessionStorage.getItem(key);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as OnboardingStatusCache;
    if (!parsed?.savedAt || Date.now() - parsed.savedAt > MAX_AGE_MS) {
      sessionStorage.removeItem(key);
      return null;
    }
    return {
      completed: Boolean(parsed.completed),
      step: Number(parsed.step) || 0,
      isOrgAdmin: Boolean(parsed.isOrgAdmin),
      isPlatformMaster: Boolean(parsed.isPlatformMaster),
    };
  } catch {
    return null;
  }
}

export function writeOnboardingStatusCache(
  status: Omit<OnboardingStatusCache, "savedAt">,
): void {
  const key = storageKey();
  if (!key) return;
  try {
    const payload: OnboardingStatusCache = { ...status, savedAt: Date.now() };
    sessionStorage.setItem(key, JSON.stringify(payload));
  } catch {
    /* ignore */
  }
}

export function clearOnboardingStatusCache(): void {
  const key = storageKey();
  if (!key) return;
  try {
    sessionStorage.removeItem(key);
  } catch {
    /* ignore */
  }
}
