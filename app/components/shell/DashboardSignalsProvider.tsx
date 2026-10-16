"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useSessionUser } from "@/app/hooks/useAppUser";
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
  anneeScolaireLabel: null,
};

/** Aligné sur le polling client (une seule requête partagée). */
export const DASHBOARD_SIGNALS_POLL_MS = 60_000;
const FOCUS_REFETCH_MIN_GAP_MS = 8_000;

function normalizeSignals(json: unknown): DashboardSignals {
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

type DashboardSignalsContextValue = {
  shortcuts: DashboardSignals["shortcuts"];
  todayNews: DashboardSignals["todayNews"];
  notifications: DashboardSignals["notifications"];
  hasCurrentWeek: boolean;
  anneeScolaireLabel: string | null;
  loading: boolean;
  /** Incrémenté à chaque fetch réseau réussi (pour callbacks). */
  fetchGeneration: number;
  refresh: () => Promise<DashboardSignals | null>;
};

const DashboardSignalsContext = createContext<DashboardSignalsContextValue | null>(null);

let fetchInFlight: Promise<DashboardSignals | null> | null = null;
let lastNetworkFetchAt = 0;

async function fetchDashboardSignalsNetwork(userId: string): Promise<DashboardSignals | null> {
  if (fetchInFlight) return fetchInFlight;
  fetchInFlight = (async () => {
    try {
      const res = await fetch("/api/dashboard/signals", { cache: "no-store" });
      if (!res.ok) return null;
      const payload = normalizeSignals(await res.json());
      writeDashboardSignalsCache(userId, payload);
      lastNetworkFetchAt = Date.now();
      return payload;
    } catch {
      return null;
    } finally {
      fetchInFlight = null;
    }
  })();
  return fetchInFlight;
}

export function DashboardSignalsProvider({ children }: { children: ReactNode }) {
  const { isLoaded, user } = useSessionUser();
  const userId = user?.id ?? null;

  const [signals, setSignals] = useState<DashboardSignals>(EMPTY_SIGNALS);
  const [loading, setLoading] = useState(true);
  const [fetchGeneration, setFetchGeneration] = useState(0);
  const warmCacheRef = useRef(false);

  useLayoutEffect(() => {
    if (!userId) return;
    const cached = readDashboardSignalsCache(userId);
    if (cached) {
      setSignals(cached);
      setLoading(false);
      warmCacheRef.current = true;
    }
  }, [userId]);

  const applyPayload = useCallback((payload: DashboardSignals | null, fromNetwork: boolean) => {
    if (!payload) return null;
    setSignals(payload);
    warmCacheRef.current = true;
    setLoading(false);
    if (fromNetwork) setFetchGeneration((g) => g + 1);
    return payload;
  }, []);

  const refresh = useCallback(
    async (opts?: { showLoading?: boolean }) => {
      if (!userId) {
        setLoading(false);
        return null;
      }
      if (opts?.showLoading && !warmCacheRef.current) {
        setLoading(true);
      }
      const payload = await fetchDashboardSignalsNetwork(userId);
      return applyPayload(payload, true);
    },
    [userId, applyPayload],
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
        const stale = Date.now() - lastNetworkFetchAt > DASHBOARD_SIGNALS_POLL_MS;
        if (stale) void refresh();
      }
    };

    const onFocus = () => {
      if (document.hidden) return;
      if (Date.now() - lastNetworkFetchAt < FOCUS_REFETCH_MIN_GAP_MS) return;
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
