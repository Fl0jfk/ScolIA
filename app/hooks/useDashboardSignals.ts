"use client";

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useUser } from "@clerk/nextjs";
import {
  readDashboardSignalsCache,
  writeDashboardSignalsCache,
} from "@/app/lib/dashboard-signals-cache";
import type { DashboardSignals } from "@/app/lib/dashboard-signals";

const EMPTY_SIGNALS: DashboardSignals = {
  shortcuts: [],
  todayNews: [],
  hasCurrentWeek: false,
  notifications: [],
};

const DEFAULT_POLL_MS = 35_000;

function normalizeSignals(json: unknown): DashboardSignals {
  const o = json && typeof json === "object" ? (json as Record<string, unknown>) : {};
  return {
    shortcuts: Array.isArray(o.shortcuts) ? (o.shortcuts as DashboardSignals["shortcuts"]) : [],
    todayNews: Array.isArray(o.todayNews) ? (o.todayNews as DashboardSignals["todayNews"]) : [],
    hasCurrentWeek: Boolean(o.hasCurrentWeek),
    notifications: Array.isArray(o.notifications)
      ? (o.notifications as DashboardSignals["notifications"])
      : [],
  };
}

type Options = {
  /** Rafraîchissement périodique (0 = désactivé). */
  pollIntervalMs?: number;
  /** Callback après chaque fetch réseau réussi. */
  onFetched?: (payload: DashboardSignals) => void;
};

/**
 * Signaux dashboard (badges, actualités, raccourcis dynamiques).
 * Affiche le cache localStorage immédiatement, puis revalide en arrière-plan.
 */
export function useDashboardSignals(options: Options = {}) {
  const { pollIntervalMs = DEFAULT_POLL_MS, onFetched } = options;
  const { isLoaded, user } = useUser();
  const userId = user?.id ?? null;

  const [signals, setSignals] = useState<DashboardSignals>(EMPTY_SIGNALS);
  const [loading, setLoading] = useState(true);
  const warmCacheRef = useRef(false);
  const onFetchedRef = useRef(onFetched);
  onFetchedRef.current = onFetched;

  useLayoutEffect(() => {
    if (!userId) return;
    const cached = readDashboardSignalsCache(userId);
    if (cached) {
      setSignals(cached);
      setLoading(false);
      warmCacheRef.current = true;
    }
  }, [userId]);

  const fetchSignals = useCallback(
    async (opts?: { showLoading?: boolean }) => {
      if (!userId) {
        setLoading(false);
        return null;
      }
      if (opts?.showLoading && !warmCacheRef.current) {
        setLoading(true);
      }
      try {
        const res = await fetch("/api/dashboard/signals", { cache: "no-store" });
        if (!res.ok) return null;
        const payload = normalizeSignals(await res.json());
        setSignals(payload);
        writeDashboardSignalsCache(userId, payload);
        warmCacheRef.current = true;
        onFetchedRef.current?.(payload);
        return payload;
      } catch {
        return null;
      } finally {
        setLoading(false);
      }
    },
    [userId],
  );

  useEffect(() => {
    if (!isLoaded) return;
    if (!userId) {
      setLoading(false);
      return;
    }
    void fetchSignals({ showLoading: !warmCacheRef.current });
  }, [isLoaded, userId, fetchSignals]);

  useEffect(() => {
    if (!isLoaded || !userId || pollIntervalMs <= 0) return;
    const id = window.setInterval(() => {
      void fetchSignals();
    }, pollIntervalMs);
    const onFocus = () => void fetchSignals();
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(id);
      window.removeEventListener("focus", onFocus);
    };
  }, [isLoaded, userId, pollIntervalMs, fetchSignals]);

  const refresh = useCallback(() => fetchSignals({ showLoading: false }), [fetchSignals]);

  return {
    shortcuts: signals.shortcuts,
    todayNews: signals.todayNews,
    notifications: signals.notifications,
    hasCurrentWeek: signals.hasCurrentWeek,
    loading,
    refresh,
  };
}
