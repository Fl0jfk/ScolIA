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
import { useAppUser } from "@/app/hooks/useAppUser";
import { clearDashboardSignalsCache } from "@/app/lib/dashboard-signals-cache";
import type { DashboardSignals } from "@/app/lib/dashboard-signals";
import type { DashboardSignalSourceId, SignalSourceStatus } from "@/app/lib/dashboard-signals-parallel";
import {
  dashboardSignalsFlightKey,
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

export const DASHBOARD_SIGNALS_POLL_MS = 60_000;
const FOCUS_REFETCH_MIN_GAP_MS = 8_000;

type DashboardSignalsContextValue = {
  shortcuts: DashboardSignals["shortcuts"];
  todayNews: DashboardSignals["todayNews"];
  notifications: DashboardSignals["notifications"];
  hasCurrentWeek: boolean;
  anneeScolaireLabel: string | null;
  sourceStatus?: Record<DashboardSignalSourceId, SignalSourceStatus>;
  loading: boolean;
  fetchGeneration: number;
  refresh: () => Promise<DashboardSignals | null>;
};

const DashboardSignalsContext = createContext<DashboardSignalsContextValue | null>(null);

type InnerProps = {
  authUserId: string;
  etablissementId: string;
  roles: string[];
  children: ReactNode;
};

function DashboardSignalsProviderInner({ authUserId, etablissementId, roles, children }: InnerProps) {
  const flightKey = dashboardSignalsFlightKey(authUserId, etablissementId);
  const sessionRef = useRef({ authUserId, etablissementId, rolesKey: roles.join("|") });
  sessionRef.current = { authUserId, etablissementId, rolesKey: roles.join("|") };

  const [signals, setSignals] = useState<DashboardSignals>(EMPTY_SIGNALS);
  const [sourceStatus, setSourceStatus] = useState<
    Record<DashboardSignalSourceId, SignalSourceStatus> | undefined
  >(undefined);
  const [loading, setLoading] = useState(true);
  const [fetchGeneration, setFetchGeneration] = useState(0);
  const warmCacheRef = useRef(false);

  useEffect(() => {
    resetDashboardSignalsSharedFetch();
    warmCacheRef.current = false;
    setFetchGeneration(0);
    setSourceStatus(undefined);

    const cached = readWarmDashboardSignalsCache(authUserId);
    if (cached) {
      setSignals(cached);
      setSourceStatus(cached.sourceStatus);
      setLoading(false);
      warmCacheRef.current = true;
    } else {
      setSignals(EMPTY_SIGNALS);
      setLoading(true);
    }

    return () => {
      resetDashboardSignalsSharedFetch();
    };
  }, [authUserId, etablissementId, roles.join("|")]);

  const applyPayload = useCallback(
    (
      payload: DashboardSignals | null,
      nextSourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
      fromNetwork: boolean,
    ) => {
      if (!payload) return null;
      if (sessionRef.current.authUserId !== authUserId) return null;
      setSignals(payload);
      setSourceStatus(nextSourceStatus);
      warmCacheRef.current = true;
      setLoading(false);
      if (fromNetwork) setFetchGeneration((g) => g + 1);
      return payload;
    },
    [authUserId],
  );

  const refresh = useCallback(
    async (opts?: { showLoading?: boolean }) => {
      const session = sessionRef.current;
      if (session.authUserId !== authUserId) return null;
      if (opts?.showLoading && !warmCacheRef.current) {
        setLoading(true);
      }
      const result = await fetchDashboardSignalsShared(flightKey, authUserId, {
        fetchJson: async () => {
          const res = await fetch("/api/dashboard/signals", { cache: "no-store" });
          if (!res.ok) throw new Error(`signals ${res.status}`);
          return res.json();
        },
      });
      if (result.flightKey !== flightKey || sessionRef.current.authUserId !== authUserId) {
        return null;
      }
      return applyPayload(result.payload, result.sourceStatus, true);
    },
    [applyPayload, authUserId, flightKey],
  );

  useEffect(() => {
    void refresh({ showLoading: !warmCacheRef.current });
  }, [authUserId, etablissementId, roles.join("|"), refresh]);

  useEffect(() => {
    const maybePoll = () => {
      if (document.hidden) return;
      void refresh();
    };

    const intervalId = window.setInterval(maybePoll, DASHBOARD_SIGNALS_POLL_MS);

    const onVisibility = () => {
      if (!document.hidden) {
        const stale = Date.now() - getDashboardSignalsLastFetchAt(flightKey) > DASHBOARD_SIGNALS_POLL_MS;
        if (stale) void refresh();
      }
    };

    const onFocus = () => {
      if (document.hidden) return;
      if (Date.now() - getDashboardSignalsLastFetchAt(flightKey) < FOCUS_REFETCH_MIN_GAP_MS) return;
      void refresh();
    };

    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("focus", onFocus);

    return () => {
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("focus", onFocus);
    };
  }, [authUserId, etablissementId, roles.join("|"), flightKey, refresh]);

  const value = useMemo<DashboardSignalsContextValue>(
    () => ({
      shortcuts: signals.shortcuts,
      todayNews: signals.todayNews,
      notifications: signals.notifications,
      hasCurrentWeek: signals.hasCurrentWeek,
      anneeScolaireLabel: signals.anneeScolaireLabel ?? null,
      sourceStatus,
      loading,
      fetchGeneration,
      refresh: () => refresh({ showLoading: false }),
    }),
    [signals, sourceStatus, loading, fetchGeneration, refresh],
  );

  return (
    <DashboardSignalsContext.Provider value={value}>{children}</DashboardSignalsContext.Provider>
  );
}

export function DashboardSignalsProvider({ children }: { children: ReactNode }) {
  const { isLoaded, user } = useAppUser();
  const authUserId = user?.id ?? null;
  const etablissementId = user?.etablissementId?.trim() || "default";
  const roles = user?.roles ?? [];

  useEffect(() => {
    if (!authUserId) {
      resetDashboardSignalsSharedFetch();
      clearDashboardSignalsCache();
    }
  }, [authUserId]);

  if (!isLoaded || !authUserId) {
    return <>{children}</>;
  }

  const sessionKey = `${authUserId}:${etablissementId}:${roles.join("|")}`;

  return (
    <DashboardSignalsProviderInner
      key={sessionKey}
      authUserId={authUserId}
      etablissementId={etablissementId}
      roles={roles}
    >
      {children}
    </DashboardSignalsProviderInner>
  );
}

export function useDashboardSignalsContext(): DashboardSignalsContextValue {
  const ctx = useContext(DashboardSignalsContext);
  if (!ctx) {
    throw new Error("useDashboardSignalsContext requires DashboardSignalsProvider");
  }
  return ctx;
}
