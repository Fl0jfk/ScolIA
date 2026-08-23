import { isListedAsRequestsStaff } from "@/app/lib/staff-directory";
import { getRequestsRoutingConfig, isListedInRouting } from "@/app/lib/requests-routing-config";

const STAFF_ROLES_FOR_REQUESTS = ["administratif","direction_ecole","direction_college","direction_lycee", "maintenance","comptabilite","education","cpe","infirmerie",] as const;

function normalizeRole(value: string) {
  return value.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/[\s-]+/g, "_");
}

function hasStaffRoleForRequests(roles: string[]): boolean {
  const normalized = roles.map(normalizeRole);
  return STAFF_ROLES_FOR_REQUESTS.some((role) => normalized.includes(role));
}

export async function canAccessRequestsStaffBoard(roles: string[], userEmail: string): Promise<boolean> {
  if (hasStaffRoleForRequests(roles)) return true;
  if (!userEmail) return false;
  try {
    const config = await getRequestsRoutingConfig();
    if (isListedInRouting(config, userEmail)) return true;
  } catch {
    /* fallback annuaire local */
  }
  return isListedAsRequestsStaff(userEmail);
}
