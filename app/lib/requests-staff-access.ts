import { hasGlobalAdminRole, hasMasterRole } from "@/app/lib/intranet-role-utils";
import { getRequestsOrgConfig } from "@/app/lib/requests-org-config";
import { isGlobalOversightManager } from "@/app/lib/requests-org-shared";
import { isListedAsRequestsStaff } from "@/app/lib/staff-directory";
import { getRequestsRoutingConfig, isListedInRouting } from "@/app/lib/requests-routing-config";
import { normalizeRequestEmail } from "@/app/lib/requests-board";
import type { CompatAuthUser } from "@/app/lib/app-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { isOrgAdminFromPublicMetadata } from "@/app/lib/intranet-auth-metadata";

const STAFF_ROLES_FOR_REQUESTS = [
  "admin",
  "administratif",
  "direction_ecole",
  "direction_college",
  "direction_lycee",
  "maintenance",
  "comptabilite",
  "surveillant",
  "cpe",
  "infirmerie",
] as const;

function normalizeRole(value: string) {
  return value.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/[\s-]+/g, "_");
}

function hasStaffRoleForRequests(roles: string[]): boolean {
  const normalized = roles.map(normalizeRole);
  return STAFF_ROLES_FOR_REQUESTS.some((role) => normalized.includes(role));
}

function isOrgUnitStaffEmail(email: string, org: Awaited<ReturnType<typeof getRequestsOrgConfig>>): boolean {
  const u = normalizeRequestEmail(email);
  if (!u) return false;
  for (const unit of org.units.filter((x) => x.active)) {
    if (unit.managerEmails.map(normalizeRequestEmail).includes(u)) return true;
    if (unit.memberEmails.map(normalizeRequestEmail).includes(u)) return true;
  }
  return false;
}

export type RequestsStaffAccessOptions = {
  orgAdmin?: boolean;
};

export async function canAccessRequestsStaffBoard(
  roles: string[],
  userEmail: string,
  options?: RequestsStaffAccessOptions,
): Promise<boolean> {
  if (options?.orgAdmin || hasGlobalAdminRole(roles) || hasMasterRole(roles)) return true;
  if (hasStaffRoleForRequests(roles)) return true;
  if (!userEmail) return false;

  try {
    const org = await getRequestsOrgConfig();
    if (isGlobalOversightManager(org, userEmail)) return true;
    if (isOrgUnitStaffEmail(userEmail, org)) return true;
  } catch {
    /* fallback routage / annuaire */
  }

  try {
    const config = await getRequestsRoutingConfig();
    if (isListedInRouting(config, userEmail)) return true;
  } catch {
    /* fallback annuaire local */
  }

  return isListedAsRequestsStaff(userEmail);
}

export function requestsStaffAccessOptionsFromUser(
  user: CompatAuthUser | null | undefined,
): RequestsStaffAccessOptions {
  return { orgAdmin: isOrgAdminFromPublicMetadata(user?.publicMetadata) };
}

export async function canAccessRequestsStaffBoardForUser(
  user: CompatAuthUser | null | undefined,
): Promise<boolean> {
  if (!user) return false;
  const roles = rolesFromUserLike(user);
  const email = user.primaryEmailAddress?.emailAddress ?? "";
  return canAccessRequestsStaffBoard(roles, email, requestsStaffAccessOptionsFromUser(user));
}
