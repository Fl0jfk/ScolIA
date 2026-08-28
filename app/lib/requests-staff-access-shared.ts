/** Helpers client-safe — accès tableau Demandes (sans server-only). */

import { INTRANET_DIRECTION_SLUGS } from "@/app/lib/intranet-roles";
import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";

const STAFF_ROLES_FOR_REQUESTS = [
  "admin",
  "administratif",
  ...INTRANET_DIRECTION_SLUGS,
  "maintenance",
  "comptabilite",
  "surveillant",
  "cpe",
  "infirmerie",
] as const;

function normalizeRole(value: string) {
  return value.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/[\s-]+/g, "_");
}

export type RequestsStaffAccessHints = {
  orgAdmin?: boolean;
  platformAdmin?: boolean;
};

/** Accès immédiat (rôles / admin org) — complété côté serveur par annuaire & config. */
export function canAccessRequestsStaffBoardSync(
  roles: string[],
  hints?: RequestsStaffAccessHints,
): boolean {
  if (hints?.orgAdmin || hints?.platformAdmin) return true;
  if (hasGlobalAdminRole(roles) || hasMasterRole(roles)) return true;
  const normalized = roles.map(normalizeRole);
  return STAFF_ROLES_FOR_REQUESTS.some((role) => normalized.includes(role));
}

export function requestsStaffAccessHintsFromClientUser(input: {
  orgAdmin?: boolean;
  platformAdmin?: boolean;
  roles?: string[];
} | null | undefined): RequestsStaffAccessHints {
  if (!input) return {};
  const roles = Array.isArray(input.roles) ? input.roles : [];
  return {
    orgAdmin: Boolean(input.orgAdmin || roles.includes("admin")),
    platformAdmin: Boolean(input.platformAdmin || roles.includes("master")),
  };
}
