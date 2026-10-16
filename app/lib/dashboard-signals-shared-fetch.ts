"use client";

import {
  readDashboardSignalsCache,
  writeDashboardSignalsCache,
} from "@/app/lib/dashboard-signals-cache";
import { isDashboardSignalsAggregateCacheable } from "@/app/lib/dashboard-signals-aggregate-cache";
import type { DashboardSignals } from "@/app/lib/dashboard-signals";
import type { DashboardSignalSourceId, SignalSourceStatus } from "@/app/lib/dashboard-signals-parallel";

export function normalizeDashboardSignalsJson(json: unknown): DashboardSignals {
  const o = json && typeof json === "object" ? (json as Record<string, unknown>) : {};
  return {
    shortcuts: Array.isArray(o.shortcuts) ? (o.shortcuts as DashboardSignals["shortcuts"]) : [],
    todayNews: Array.isArray(o.todayNews) ? (o.todayNews as DashboardSignals["todayNews"]) : [],
    hasCurrentWeek: Boolean(o.hasCurrentWeek),
    notifications: Array.isArray(o.notifications)
      ? (o.notifications as DashboardSignals["notifications"])
      : [],
    anneeScolaireLabel:
      typeof o.anneeScolaireLabel === "string" ? o.anneeScolaireLabel : null,
  };
}

function parseSourceStatus(json: unknown): Record<DashboardSignalSourceId, SignalSourceStatus> | undefined {
  if (!json || typeof json !== "object") return undefined;
  const perf = (json as { perf?: { sourceStatus?: unknown } }).perf;
  const raw = perf?.sourceStatus;
  if (!raw || typeof raw !== "object") return undefined;
  return raw as Record<DashboardSignalSourceId, SignalSourceStatus>;
}

export type SharedFetchResult = {
  userId: string;
  payload: DashboardSignals | null;
  wroteClientCache: boolean;
};

type InFlightRecord = {
  userId: string;
  promise: Promise<SharedFetchResult>;
};

let inFlight: InFlightRecord | null = null;
let lastNetworkFetchAt = 0;

export function getDashboardSignalsLastFetchAt(): number {
  return lastNetworkFetchAt;
}

/** À appeler quand l’utilisateur session change (sign-out / autre compte). */
export function resetDashboardSignalsSharedFetch(): void {
  inFlight = null;
  lastNetworkFetchAt = 0;
}

export type FetchDashboardSignalsDeps = {
  fetchJson: () => Promise<unknown>;
};

/**
 * Un seul fetch réseau partagé par userId ; les résultats d’un autre user ou d’une requête
 * orpheline après changement de session sont ignorés (pas de cache client).
 */
export async function fetchDashboardSignalsShared(
  userId: string,
  deps: FetchDashboardSignalsDeps,
): Promise<SharedFetchResult> {
  if (inFlight?.userId === userId) {
    return inFlight.promise;
  }

  const startedFor = userId;
  const promise = (async (): Promise<SharedFetchResult> => {
    try {
      const json = await deps.fetchJson();
      if (inFlight?.userId !== startedFor) {
        return { userId: startedFor, payload: null, wroteClientCache: false };
      }
      const payload = normalizeDashboardSignalsJson(json);
      const sourceStatus = parseSourceStatus(json);
      const cacheable = isDashboardSignalsAggregateCacheable(sourceStatus);
      let wroteClientCache = false;
      if (cacheable) {
        writeDashboardSignalsCache(startedFor, payload);
        wroteClientCache = true;
      }
      lastNetworkFetchAt = Date.now();
      return { userId: startedFor, payload, wroteClientCache };
    } catch {
      return { userId: startedFor, payload: null, wroteClientCache: false };
    } finally {
      if (inFlight?.userId === startedFor) {
        inFlight = null;
      }
    }
  })();

  inFlight = { userId: startedFor, promise };
  return promise;
}

export function readWarmDashboardSignalsCache(userId: string): DashboardSignals | null {
  return readDashboardSignalsCache(userId);
}
