"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
import type { DashboardSignals } from "@/app/lib/dashboard-signals";
import {
  fetchDashboardSignalsShared,
  getDashboardSignalsLastFetchAt,
  readWarmDashboardSignalsCache,
  resetDashboardSignalsSharedFetch,
} from "@/app/lib/dashboard-signals-shared-fetch";

const EMPTY_SIGNALS: DashboardSignals = {
  shortcuts: [],
  todayNews: [],
  hasCurrentWeek: false,
  notifications: [],
  anneeScolaireLabel: null,
};

/** Aligné sur le polling client (une seule requête partagée). */
export const DASHBOARD_SIGNALS_POLL_MS = 60_000;
const FOCUS_REFETCH_MIN_GAP_MS = 8_000;

type DashboardSignalsContextValue = {
  shortcuts: DashboardSignals["shortcuts"];
  todayNews: DashboardSignals["todayNews"];
  notifications: DashboardSignals["notifications"];
  hasCurrentWeek: boolean;
  anneeScolaireLabel: string | null;
  loading: boolean;
  fetchGeneration: number;
  refresh: () => Promise<DashboardSignals | null>;
};

const DashboardSignalsContext = createContext<DashboardSignalsContextValue | null>(null);

export function DashboardSignalsProvider({ children }: { children: ReactNode }) {
  const { isLoaded, user } = useSessionUser();
  const userId = user?.id ?? null;
  const userIdRef = useRef(userId);
  userIdRef.current = userId;

  const [signals, setSignals] = useState<DashboardSignals>(EMPTY_SIGNALS);
  const [loading, setLoading] = useState(true);
  const [fetchGeneration, setFetchGeneration] = useState(0);
  const warmCacheRef = useRef(false);

  useEffect(() => {
    resetDashboardSignalsSharedFetch();
    warmCacheRef.current = false;
    setFetchGeneration(0);

    if (!userId) {
      setSignals(EMPTY_SIGNALS);
      setLoading(false);
      return;
    }

    const cached = readWarmDashboardSignalsCache(userId);
    if (cached) {
      setSignals(cached);
      setLoading(false);
      warmCacheRef.current = true;
    } else {
      setSignals(EMPTY_SIGNALS);
      setLoading(true);
    }
  }, [userId]);

  const applyPayload = useCallback((payload: DashboardSignals | null, fromNetwork: boolean) => {
    if (!payload) return null;
    if (userIdRef.current !== userId) return null;
    setSignals(payload);
    warmCacheRef.current = true;
    setLoading(false);
    if (fromNetwork) setFetchGeneration((g) => g + 1);
    return payload;
  }, [userId]);

  const refresh = useCallback(
    async (opts?: { showLoading?: boolean }) => {
      const activeUserId = userIdRef.current;
      if (!activeUserId) {
        setLoading(false);
        return null;
      }
      if (opts?.showLoading && !warmCacheRef.current) {
        setLoading(true);
      }
      const result = await fetchDashboardSignalsShared(activeUserId, {
        fetchJson: async () => {
          const res = await fetch("/api/dashboard/signals", { cache: "no-store" });
          if (!res.ok) throw new Error(`signals ${res.status}`);
          return res.json();
        },
      });
      if (result.userId !== activeUserId || userIdRef.current !== activeUserId) {
        return null;
      }
      return applyPayload(result.payload, true);
    },
    [applyPayload],
  );

  useEffect(() => {
    if (!isLoaded) return;
    if (!userId) {
      setLoading(false);
      return;
    }
    void refresh({ showLoading: !warmCacheRef.current });
  }, [isLoaded, userId, refresh]);

  useEffect(() => {
    if (!isLoaded || !userId) return;

    const maybePoll = () => {
      if (document.hidden) return;
      void refresh();
    };

    const intervalId = window.setInterval(maybePoll, DASHBOARD_SIGNALS_POLL_MS);

    const onVisibility = () => {
      if (!document.hidden) {
        const stale = Date.now() - getDashboardSignalsLastFetchAt() > DASHBOARD_SIGNALS_POLL_MS;
        if (stale) void refresh();
      }
    };

    const onFocus = () => {
      if (document.hidden) return;
      if (Date.now() - getDashboardSignalsLastFetchAt() < FOCUS_REFETCH_MIN_GAP_MS) return;
      void refresh();
    };

    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("focus", onFocus);

    return () => {
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("focus", onFocus);
    };
  }, [isLoaded, userId, refresh]);

  const value = useMemo<DashboardSignalsContextValue>(
    () => ({
      shortcuts: signals.shortcuts,
      todayNews: signals.todayNews,
      notifications: signals.notifications,
      hasCurrentWeek: signals.hasCurrentWeek,
      anneeScolaireLabel: signals.anneeScolaireLabel ?? null,
      loading,
      fetchGeneration,
      refresh: () => refresh({ showLoading: false }),
    }),
    [signals, loading, fetchGeneration, refresh],
  );

  return (
    <DashboardSignalsContext.Provider value={value}>{children}</DashboardSignalsContext.Provider>
  );
}

export function useDashboardSignalsContext(): DashboardSignalsContextValue {
  const ctx = useContext(DashboardSignalsContext);
  if (!ctx) {
    throw new Error("useDashboardSignalsContext requires DashboardSignalsProvider");
  }
  return ctx;
}
