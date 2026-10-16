import {
  DASHBOARD_SIGNAL_SOURCES,
  type DashboardSignalSourceId,
  type SignalSourceStatus,
} from "@/app/lib/dashboard-signals-parallel";

/** Agrégat dashboard : ne pas mettre en cache si une source a échoué ou timeout. */
export function isDashboardSignalsAggregateCacheable(
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): boolean {
  if (!sourceStatus) return false;
  return DASHBOARD_SIGNAL_SOURCES.every((id) => sourceStatus[id] === "ok");
}
