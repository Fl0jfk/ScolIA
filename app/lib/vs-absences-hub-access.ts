/**
 * Droits hub « Absences » (/vie-scolaire/absences) — utilisable côté client.
 * Proxy démo CPE Lot 2 : rôles direction (ex. florian+direction@h-me.fr).
 */
import { isAnyDirectionRole } from "@/app/lib/establishment-catalog";
import { hasRole } from "@/app/lib/intranet-role-utils";

export function canConsultAbsencesHubTab(opts: {
  isOrgAdmin: boolean;
  accessibleModuleIds: Set<string> | null | undefined;
  roles: string[];
}): boolean {
  if (opts.isOrgAdmin) return true;
  if (opts.accessibleModuleIds?.has("absences-accueil-consultation")) return true;
  /** Suivi absences élèves (appel → CPE) — même lecture que CPE. */
  if (opts.accessibleModuleIds?.has("vs-absences")) return true;
  if (opts.accessibleModuleIds != null) return false;

  return (
    isAnyDirectionRole(opts.roles) ||
    hasRole(opts.roles, "cpe") ||
    hasRole(opts.roles, "surveillant") ||
    hasRole(opts.roles, "administratif")
  );
}
