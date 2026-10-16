import {
  notificationCountForPillar,
  notificationCountForShortcut,
  type DashboardNotification,
} from "@/app/lib/dashboard-signals";
import { moduleIdToPillarId, type DashboardPillarId } from "@/app/lib/dashboard-pillars";
import { resolveDashboardSignalSource } from "@/app/lib/dashboard-signal-source";
import type { DashboardSignalSourceId, SignalSourceStatus } from "@/app/lib/dashboard-signals-parallel";

export type DashboardNotificationBadgeProps = {
  count?: number;
  unavailable?: boolean;
  /** Au moins un compteur du groupe est indisponible alors que d’autres sont connus. */
  partialUnavailable?: boolean;
  compact?: boolean;
};

export type DashboardSignalCountable = {
  id: string;
  moduleId: string;
  signalSourceId?: DashboardSignalSourceId;
};

export function isDashboardSignalSourceUnavailable(
  sourceId: DashboardSignalSourceId,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): boolean {
  if (!sourceStatus) return false;
  const st = sourceStatus[sourceId];
  return st === "timeout" || st === "error";
}

export function isDashboardNotificationUnavailable(
  item: DashboardSignalCountable,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): boolean {
  const src = resolveDashboardSignalSource(item);
  return isDashboardSignalSourceUnavailable(src, sourceStatus);
}

/** Somme des comptes dont la source est OK. */
export function sumAvailableNotificationCounts(
  items: Array<Pick<DashboardNotification, "count"> & DashboardSignalCountable>,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): number {
  return items.reduce((sum, item) => {
    if (isDashboardNotificationUnavailable(item, sourceStatus)) return sum;
    return sum + (item.count > 0 ? item.count : 0);
  }, 0);
}

export function dashboardNotificationBadgeProps(
  count: number,
  item: DashboardSignalCountable,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
  opts?: { compact?: boolean },
): DashboardNotificationBadgeProps | null {
  if (isDashboardNotificationUnavailable(item, sourceStatus)) {
    return { unavailable: true, compact: opts?.compact };
  }
  if (count <= 0) return null;
  return count > 99 ? { count: 99, partialUnavailable: false } : { count };
}

export function dashboardPillarBadgeProps(
  pillarId: DashboardPillarId,
  notifications: DashboardNotification[],
  shortcuts: Array<
    { id: string; moduleId: string; pillarId?: DashboardPillarId; signalSourceId?: DashboardSignalSourceId }
  >,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
  opts?: { compact?: boolean },
): DashboardNotificationBadgeProps | null {
  const pillarNotifs = notifications.filter(
    (n) => moduleIdToPillarId(n.moduleId) === pillarId,
  );
  let unavailable = false;
  let count = 0;
  for (const n of pillarNotifs) {
    if (isDashboardNotificationUnavailable(n, sourceStatus)) {
      unavailable = true;
    } else if (n.count > 0) {
      count += n.count;
    }
  }
  for (const s of shortcuts) {
    const inPillar =
      s.pillarId === pillarId || moduleIdToPillarId(s.moduleId) === pillarId;
    if (!inPillar) continue;
    const c = notificationCountForShortcut(s, notifications);
    if (isDashboardNotificationUnavailable(s, sourceStatus)) {
      unavailable = true;
    } else if (c > 0) {
      count += c;
    }
  }
  if (unavailable) {
    return { unavailable: true, compact: opts?.compact };
  }
  if (count <= 0) return null;
  return count > 99 ? { count: 99 } : { count };
}

/** État KPI bento : indisponible (pas de chiffre partiel). */
export function isDashboardFaceCountUnavailable(
  count: number,
  item: DashboardSignalCountable,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): boolean {
  return isDashboardNotificationUnavailable(item, sourceStatus);
}
