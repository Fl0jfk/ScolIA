"use client";

import { useEffect, useRef } from "react";
import {
  useDashboardSignalsContext,
  DASHBOARD_SIGNALS_POLL_MS,
} from "@/app/components/shell/DashboardSignalsProvider";
import type { DashboardSignals } from "@/app/lib/dashboard-signals";

export { DASHBOARD_SIGNALS_POLL_MS };

type Options = {
  /** @deprecated Polling global 60 s (onglet visible). Ignoré. */
  pollIntervalMs?: number;
  onFetched?: (payload: DashboardSignals) => void;
};

/**
 * Signaux dashboard — store partagé (sidebar, home, piliers) : un fetch, polling 60 s.
 */
export function useDashboardSignals(options: Options = {}) {
  const { onFetched } = options;
  const ctx = useDashboardSignalsContext();
  const onFetchedRef = useRef(onFetched);
  onFetchedRef.current = onFetched;
  const lastGen = useRef(0);

  useEffect(() => {
    if (!onFetchedRef.current || ctx.fetchGeneration === 0) return;
    if (ctx.fetchGeneration === lastGen.current) return;
    lastGen.current = ctx.fetchGeneration;
    onFetchedRef.current({
      shortcuts: ctx.shortcuts,
      todayNews: ctx.todayNews,
      hasCurrentWeek: ctx.hasCurrentWeek,
      notifications: ctx.notifications,
      anneeScolaireLabel: ctx.anneeScolaireLabel,
    });
  }, [ctx.fetchGeneration, ctx.shortcuts, ctx.todayNews, ctx.hasCurrentWeek, ctx.notifications, ctx.anneeScolaireLabel]);

  return {
    shortcuts: ctx.shortcuts,
    todayNews: ctx.todayNews,
    notifications: ctx.notifications,
    hasCurrentWeek: ctx.hasCurrentWeek,
    anneeScolaireLabel: ctx.anneeScolaireLabel,
    loading: ctx.loading,
    refresh: ctx.refresh,
  };
}
