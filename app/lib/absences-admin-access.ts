import type { Establishment, NotificationsConfig } from "@/app/lib/app-config-schemas";
import type { AbsenceRecord } from "@/app/lib/absences-types";
import { requiresProcessorAfterValidation } from "@/app/lib/absence-hours-treatment";
import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";
import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import {
  collectAbsenceProcessors,
  isConfiguredAbsenceProcessor,
  viewerIsConfiguredOgecAbsenceValidator,
} from "@/app/lib/absences-validation-recipients";

export type AbsenceProcessorViewer = {
  email?: string | null;
  userId?: string | null;
  roles?: string[];
};

/**
 * File rectorat / RH : validée par la direction, pas encore close administrativement.
 * Professeurs : uniquement les dossiers à déclarer (rectorat / ONISE) — pas le rattrapage interne.
 * OGEC : inchangé (toute absence validée reste chez la RH).
 */
export function isAbsencePendingForProcessor(abs: AbsenceRecord): boolean {
  if (abs.managerDecision !== "VALIDEE") return false;
  if (abs.workflowStatus === "CLOTUREE") return false;
  if (abs.source === "admin_manual" || abs.source === "admin_pdf") return false;
  return requiresProcessorAfterValidation(abs);
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

/** Onglet Direction : rôle direction, admin, ou validateur OGEC nominatif. */
export function viewerCanSeeAbsenceDirectionQueue(
  viewer: AbsenceProcessorViewer,
  notifications: NotificationsConfig | null | undefined,
): boolean {
  const roles = viewer.roles || [];
  if (hasGlobalAdminRole(roles) || hasMasterRole(roles) || isAnyDirectionRole(roles)) return true;
  return viewerIsConfiguredOgecAbsenceValidator(viewer, notifications);
}
