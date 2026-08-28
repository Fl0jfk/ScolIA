import "server-only";

import { getAppSession } from "@/app/lib/app-session";
import { getRequestsOrgConfig } from "@/app/lib/requests-org-config";
import { isGlobalOversightManager } from "@/app/lib/requests-org-shared";
import { isListedAsRequestsStaff } from "@/app/lib/staff-directory";
import { getRequestsRoutingConfig, isListedInRouting } from "@/app/lib/requests-routing-config";
import { normalizeRequestEmail } from "@/app/lib/requests-board";
import type { CompatAuthUser } from "@/app/lib/app-session";
import { rolesFromUserLike } from "@/app/lib/intranet-roles";
import { isOrgAdminFromPublicMetadata } from "@/app/lib/intranet-auth-metadata";
import {
  canAccessRequestsStaffBoardSync,
  requestsStaffAccessHintsFromClientUser,
  type RequestsStaffAccessHints,
} from "@/app/lib/requests-staff-access-shared";

export type { RequestsStaffAccessHints };

function isOrgUnitStaffEmail(email: string, org: Awaited<ReturnType<typeof getRequestsOrgConfig>>): boolean {
  const u = normalizeRequestEmail(email);
  if (!u) return false;
  for (const unit of org.units.filter((x) => x.active)) {
    if (unit.managerEmails.map(normalizeRequestEmail).includes(u)) return true;
    if (unit.memberEmails.map(normalizeRequestEmail).includes(u)) return true;
  }
  return false;
}

export type RequestsStaffAccessOptions = RequestsStaffAccessHints;

export async function canAccessRequestsStaffBoard(
  roles: string[],
  userEmail: string,
  options?: RequestsStaffAccessOptions,
): Promise<boolean> {
  if (canAccessRequestsStaffBoardSync(roles, options)) return true;
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
  const meta = user?.publicMetadata;
  const roles = rolesFromUserLike(user);
  return {
    orgAdmin: isOrgAdminFromPublicMetadata(meta),
    platformAdmin: Boolean((meta as Record<string, unknown> | undefined)?.platform_admin),
    ...requestsStaffAccessHintsFromClientUser({ roles, orgAdmin: isOrgAdminFromPublicMetadata(meta) }),
  };
}

async function resolveStaffAccessContext(user: CompatAuthUser | null | undefined): Promise<{
  roles: string[];
  email: string;
  options: RequestsStaffAccessOptions;
}> {
  const session = await getAppSession();
  if (session?.user) {
    const u = session.user;
    return {
      roles: u.roles,
      email: u.email,
      options: requestsStaffAccessHintsFromClientUser({
        roles: u.roles,
        orgAdmin: u.orgAdmin,
        platformAdmin: u.platformAdmin,
      }),
    };
  }
  return {
    roles: rolesFromUserLike(user),
    email: user?.primaryEmailAddress?.emailAddress ?? "",
    options: requestsStaffAccessOptionsFromUser(user),
  };
}

export async function canAccessRequestsStaffBoardForUser(
  user: CompatAuthUser | null | undefined,
): Promise<boolean> {
  const ctx = await resolveStaffAccessContext(user);
  return canAccessRequestsStaffBoard(ctx.roles, ctx.email, ctx.options);
}

export async function canAccessRequestsStaffBoardFromSession(): Promise<boolean> {
  const session = await getAppSession();
  if (!session?.user) return false;
  const u = session.user;
  return canAccessRequestsStaffBoard(
    u.roles,
    u.email,
    requestsStaffAccessHintsFromClientUser({
      roles: u.roles,
      orgAdmin: u.orgAdmin,
      platformAdmin: u.platformAdmin,
    }),
  );
}
