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

/** NJ-31 (D-053) : le personnel mobile doit aussi avoir le module (même matrice que l'intranet). */
export async function mobileStaffHasModule(
  ctx: MobileStaffContext,
  moduleId: string,
): Promise<boolean> {
  const { getIntranetModuleById, rolesAllowModule } = await import("@/app/lib/intranet-modules");
  const { loadModuleAccess } = await import("@/app/lib/module-access-store");
  const mod = getIntranetModuleById(moduleId);
  if (!mod) return false;
  const access = await loadModuleAccess();
  const isOrgAdmin = ctx.roles.includes("admin") || ctx.roles.includes("master");
  return rolesAllowModule(ctx.roles, mod, isOrgAdmin, access, {
    userId: ctx.authUserId,
    businessUserId: ctx.authUserId,
  });
}
