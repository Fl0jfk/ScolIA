import {
  notificationCountForPillar,
  notificationCountForShortcut,
  type DashboardNotification,
} from "@/app/lib/dashboard-signals";
import { moduleIdToPillarId, type DashboardPillarId } from "@/app/lib/dashboard-pillars";
import type { DashboardSignalSourceId, SignalSourceStatus } from "@/app/lib/dashboard-signals-parallel";

export type DashboardNotificationBadgeProps = {
  count?: number;
  label?: string;
};

export function dashboardSignalSourceForNotification(item: {
  id: string;
  moduleId: string;
}): DashboardSignalSourceId {
  const { id, moduleId } = item;
  if (moduleId === "stages" || id.startsWith("stages-")) return "stages";
  if (
    moduleId === "demandes-hse" ||
    moduleId === "requests-staff" ||
    moduleId === "photocopies-couleur"
  ) {
    return "demandes";
  }
  if (moduleId === "documents" || id.includes("documents")) return "documents";
  if (moduleId === "internat") return "internat";
  if (moduleId === "vs-sanctions" || moduleId === "vs-carnet" || id.includes("sanction")) {
    return "others";
  }
  if (
    moduleId === "vs-absences" ||
    moduleId === "vs-appels" ||
    moduleId === "vs-calendrier" ||
    id.includes("vs-") ||
    id.includes("absence")
  ) {
    return "absences";
  }
  if (moduleId === "travels" || id.includes("travels") || id.includes("creneaux")) {
    return "others";
  }
  if (moduleId === "rh" || moduleId === "absences" || moduleId === "mon-planning") {
    return "rh";
  }
  if (moduleId === "eleve-dossier" || moduleId === "facturation") return "others";
  return "others";
}

export function isDashboardNotificationUnavailable(
  item: Pick<DashboardNotification, "id" | "moduleId">,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): boolean {
  if (!sourceStatus) return false;
  const src = dashboardSignalSourceForNotification(item);
  const st = sourceStatus[src];
  return st === "timeout" || st === "error";
}

/** Compteur affiché : « indisponible » si la source a échoué, sinon le nombre. */
export function formatDashboardNotificationCount(
  count: number,
  item: Pick<DashboardNotification, "id" | "moduleId">,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): string {
  if (isDashboardNotificationUnavailable(item, sourceStatus)) return "indisponible";
  if (count <= 0) return "0";
  return count > 99 ? "99+" : String(count);
}

export function formatDashboardNumericBadge(
  count: number,
  sourceId: DashboardSignalSourceId,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): string | null {
  if (sourceStatus && (sourceStatus[sourceId] === "timeout" || sourceStatus[sourceId] === "error")) {
    return "indisponible";
  }
  if (count <= 0) return null;
  return count > 99 ? "99+" : String(count);
}

/** Props pour `NotificationCountBadge` (compteur ou pastille « indisponible »). */
export function dashboardNotificationBadgeProps(
  count: number,
  item: Pick<DashboardNotification, "id" | "moduleId">,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): DashboardNotificationBadgeProps | null {
  const formatted = formatDashboardNotificationCount(count, item, sourceStatus);
  if (formatted === "indisponible") return { label: "indisponible" };
  if (formatted === "0") return null;
  if (formatted === "99+") return { label: "99+" };
  const n = Number(formatted);
  return Number.isFinite(n) ? { count: n } : { label: formatted };
}

export function dashboardPillarBadgeProps(
  pillarId: DashboardPillarId,
  notifications: DashboardNotification[],
  shortcuts: Array<{ id: string; moduleId: string; pillarId?: DashboardPillarId }>,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): DashboardNotificationBadgeProps | null {
  const count = notificationCountForPillar(pillarId, notifications);
  if (count > 0) {
    return count > 99 ? { label: "99+" } : { count };
  }
  for (const n of notifications) {
    if (moduleIdToPillarId(n.moduleId) !== pillarId) continue;
    if (isDashboardNotificationUnavailable(n, sourceStatus)) {
      return { label: "indisponible" };
    }
  }
  for (const s of shortcuts) {
    const inPillar =
      s.pillarId === pillarId || moduleIdToPillarId(s.moduleId) === pillarId;
    if (!inPillar) continue;
    const c = notificationCountForShortcut(s, notifications);
    if (c === 0 && isDashboardNotificationUnavailable(s, sourceStatus)) {
      return { label: "indisponible" };
    }
  }
  return null;
}

/** Libellé grand compteur (bento home / hub pilier). */
export function dashboardFaceCountLabel(
  count: number,
  item: Pick<DashboardNotification, "id" | "moduleId">,
  sourceStatus: Record<DashboardSignalSourceId, SignalSourceStatus> | undefined,
): string | null {
  const formatted = formatDashboardNotificationCount(count, item, sourceStatus);
  if (formatted === "0") return null;
  return formatted;
}
