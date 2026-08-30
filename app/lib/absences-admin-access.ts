import type { Establishment, NotificationsConfig } from "@/app/lib/app-config-schemas";
import type { AbsenceRecord } from "@/app/lib/absences-types";
import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";
import {
  collectAbsenceProcessors,
  isConfiguredAbsenceProcessor,
} from "@/app/lib/absences-validation-recipients";

export type AbsenceProcessorViewer = {
  email?: string | null;
  userId?: string | null;
  roles?: string[];
};

/** File rectorat / RH : validée par la direction, pas encore close administrativement. */
export function isAbsencePendingForProcessor(abs: AbsenceRecord): boolean {
  if (abs.managerDecision !== "VALIDEE") return false;
  if (abs.workflowStatus === "CLOTUREE") return false;
  if (abs.source === "admin_manual" || abs.source === "admin_pdf") return false;
  return true;
}

/** Le traiteur (rectorat / RH) voit le dossier une fois la direction passée. */
export function processorMayAccessValidatedAbsence(
  abs: AbsenceRecord,
  viewer: AbsenceProcessorViewer,
  notifications: NotificationsConfig | null | undefined,
  establishments: Establishment[],
): boolean {
  if (abs.managerDecision !== "VALIDEE") return false;
  return viewerIsAbsenceProcessor(abs, viewer, notifications, establishments);
}

export function viewerIsAbsenceProcessor(
  abs: Pick<AbsenceRecord, "data" | "createdBy">,
  viewer: AbsenceProcessorViewer,
  notifications: NotificationsConfig | null | undefined,
  establishments: Establishment[],
): boolean {
  const roles = viewer.roles || [];
  if (hasGlobalAdminRole(roles) || hasMasterRole(roles)) return true;
  if (!notifications) return false;
  const processors = collectAbsenceProcessors(abs, notifications, establishments);
  const email = String(viewer.email || "").trim().toLowerCase();
  const userId = String(viewer.userId || "").trim();
  return processors.some((p) => {
    if (email && p.email === email) return true;
    if (userId && p.userId && p.userId === userId) return true;
    return false;
  });
}

export function viewerCanSeeProcessorQueue(
  viewer: AbsenceProcessorViewer,
  notifications: NotificationsConfig | null | undefined,
): boolean {
  const roles = viewer.roles || [];
  if (hasGlobalAdminRole(roles) || hasMasterRole(roles)) return true;
  if (!notifications) return false;
  return isConfiguredAbsenceProcessor(viewer, notifications);
}

export function viewerCanConfigureAbsenceProcessors(roles: string[]): boolean {
  if (hasGlobalAdminRole(roles) || hasMasterRole(roles)) return true;
  return roles.some((r) => {
    const n = String(r)
      .toLowerCase()
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "");
    return n.includes("direction");
  });
}
