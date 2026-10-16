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

/** Écriture agrégat Valkey : sources ok et pas d’invalidation depuis le début de la requête. */
export function shouldWriteDashboardSignalsAggregateCache(
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
  bundleGenerationAtRequestStart: number,
  bundleGenerationNow: number,
): boolean {
  if (!isDashboardSignalsAggregateCacheable(sourceStatus)) return false;
  return bundleGenerationAtRequestStart === bundleGenerationNow;
}
