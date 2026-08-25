import "server-only";

import { NextResponse } from "next/server";
import { requireAppUser } from "@/app/lib/app-session";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { requireTenantId } from "@/app/lib/tenant-scope";
import {
  hasStaffCapableRole,
  isFamilyOnlyRoleSet,
} from "@/app/lib/channel-access";
import { listStaffMembershipsForUser } from "@/app/lib/user-membership";

export type MobileStaffContext = {
  authUserId: string;
  email: string;
  name: string;
  etablissementId: string;
  roles: string[];
};

/**
 * Staff-lite app : personnel uniquement (pas parent/élève pur).
 * Sous-ensemble volontairement petit — pas un proxy de l’intranet.
 */
export async function requireMobileStaffAccess(): Promise<
  { ok: true; ctx: MobileStaffContext } | { ok: false; response: NextResponse }
> {
  const appUser = await requireAppUser();
  if (!appUser.ok) {
    return {
      ok: false,
      response: NextResponse.json({ error: "Non autorisé.", code: "AUTH_REQUIRED" }, { status: 401 }),
    };
  }

  const tenant = await requireTenantId();
  if (!tenant.ok) return tenant;

  const { etablissementId, authUserId } = tenant.ctx;
  const roles = await listUserRolesFromDb(authUserId, etablissementId);
  const staffMemberships = await listStaffMembershipsForUser(authUserId);
  const hasStaffHere = staffMemberships.some((m) => m.etablissementId === etablissementId);

  if (isFamilyOnlyRoleSet(roles) || (!hasStaffCapableRole(roles) && !hasStaffHere)) {
    return {
      ok: false,
      response: NextResponse.json(
        {
          error: "Réservé au personnel (mode app lite).",
          code: "MOBILE_STAFF_FORBIDDEN",
        },
        { status: 403 },
      ),
    };
  }

  return {
    ok: true,
    ctx: {
      authUserId,
      email: appUser.user.email,
      name: appUser.user.name || appUser.user.email,
      etablissementId,
      roles,
    },
  };
}
