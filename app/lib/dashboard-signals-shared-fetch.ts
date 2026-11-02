"use client";

import {
  readDashboardSignalsCache,
  writeDashboardSignalsCache,
  type DashboardSignalsCachePayload,
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

export function parseDashboardSignalsSourceStatus(
  json: unknown,
): Record<DashboardSignalSourceId, SignalSourceStatus> | undefined {
  if (!json || typeof json !== "object") return undefined;
  const perf = (json as { perf?: { sourceStatus?: unknown } }).perf;
  const raw = perf?.sourceStatus;
  if (!raw || typeof raw !== "object") return undefined;
  return raw as Record<DashboardSignalSourceId, SignalSourceStatus>;
}

export type SharedFetchResult = {
  flightKey: string;
  payload: DashboardSignals | null;
  sourceStatus?: Record<DashboardSignalSourceId, SignalSourceStatus>;
  wroteClientCache: boolean;
};

const inFlightByKey = new Map<string, Promise<SharedFetchResult>>();
const lastFetchAtByFlight = new Map<string, number>();
let fetchEpoch = 0;

export function dashboardSignalsFlightKey(userId: string, etablissementId: string): string {
  return `${userId.trim()}:${etablissementId.trim() || "default"}`;
}

export function getDashboardSignalsLastFetchAt(flightKey: string): number {
  return lastFetchAtByFlight.get(flightKey) ?? 0;
}

/** À appeler quand l’utilisateur / établissement session change. */
export function resetDashboardSignalsSharedFetch(): void {
  fetchEpoch += 1;
  inFlightByKey.clear();
  lastFetchAtByFlight.clear();
}

export type FetchDashboardSignalsDeps = {
  fetchJson: () => Promise<unknown>;
};

export async function fetchDashboardSignalsShared(
  flightKey: string,
  userId: string,
  deps: FetchDashboardSignalsDeps,
): Promise<SharedFetchResult> {
  const existing = inFlightByKey.get(flightKey);
  if (existing) {
    return existing;
  }

  const startedKey = flightKey;
  const startedEpoch = fetchEpoch;
  const promise = (async (): Promise<SharedFetchResult> => {
    try {
      const json = await deps.fetchJson();
      if (startedEpoch !== fetchEpoch) {
        return { flightKey: startedKey, payload: null, wroteClientCache: false };
      }
      const payload = normalizeDashboardSignalsJson(json);
      const sourceStatus = parseDashboardSignalsSourceStatus(json);
      const cacheable = isDashboardSignalsAggregateCacheable(sourceStatus);
      let wroteClientCache = false;
      if (cacheable) {
        writeDashboardSignalsCache(userId, { ...payload, sourceStatus });
        wroteClientCache = true;
      }
      lastFetchAtByFlight.set(startedKey, Date.now());
      return { flightKey: startedKey, payload, sourceStatus, wroteClientCache };
    } catch {
      return { flightKey: startedKey, payload: null, wroteClientCache: false };
    } finally {
      inFlightByKey.delete(startedKey);
    }
  })();

  inFlightByKey.set(startedKey, promise);
  return promise;
}

export function readWarmDashboardSignalsCache(userId: string): DashboardSignalsCachePayload | null {
  return readDashboardSignalsCache(userId);
}
