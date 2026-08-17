import type { DashboardSignals } from "@/app/lib/dashboard-signals";

const STORAGE_KEY_PREFIX = "scola.dashboard-signals:v1:";
/** Au-delà : le cache n'est plus affiché (données trop vieilles). */
const MAX_AGE_MS = 10 * 60 * 1000;

type DashboardSignalsCache = {
  payload: DashboardSignals;
  savedAt: number;
};

function storageKey(userId: string): string | null {
  if (typeof window === "undefined" || !userId.trim()) return null;
  return `${STORAGE_KEY_PREFIX}${window.location.hostname}:${userId.trim()}`;
}

export function readDashboardSignalsCache(userId: string): DashboardSignals | null {
  const key = storageKey(userId);
  if (!key) return null;
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as DashboardSignalsCache;
    if (!parsed?.savedAt || Date.now() - parsed.savedAt > MAX_AGE_MS) {
      localStorage.removeItem(key);
      return null;
    }
    const p = parsed.payload;
    if (!p || typeof p !== "object") return null;
    return {
      shortcuts: Array.isArray(p.shortcuts) ? p.shortcuts : [],
      todayNews: Array.isArray(p.todayNews) ? p.todayNews : [],
      hasCurrentWeek: Boolean(p.hasCurrentWeek),
      notifications: Array.isArray(p.notifications) ? p.notifications : [],
    };
  } catch {
    return null;
  }
}

export function writeDashboardSignalsCache(userId: string, payload: DashboardSignals): void {
  const key = storageKey(userId);
  if (!key) return;
  try {
    const entry: DashboardSignalsCache = { payload, savedAt: Date.now() };
    localStorage.setItem(key, JSON.stringify(entry));
  } catch {
    /* quota / mode privé */
  }
}

export function clearDashboardSignalsCache(userId?: string): void {
  if (typeof window === "undefined") return;
  try {
    if (userId?.trim()) {
      const key = storageKey(userId);
      if (key) localStorage.removeItem(key);
      return;
    }
    const prefix = `${STORAGE_KEY_PREFIX}${window.location.hostname}:`;
    for (let i = localStorage.length - 1; i >= 0; i--) {
      const k = localStorage.key(i);
      if (k?.startsWith(prefix)) localStorage.removeItem(k);
    }
  } catch {
    /* ignore */
  }
}
