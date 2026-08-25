import "server-only";

import type { NextRequest } from "next/server";
import { and, eq } from "drizzle-orm";
import { isBetterAuthActive } from "@/app/lib/auth-config";
import { getBetterAuth } from "@/app/lib/auth-server";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";
import { roleRequiresTwoFactor } from "@/app/lib/two-factor-policy";
import type { TenantConfig } from "@/app/lib/tenant-types";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user } from "@/db/schema";

export type BetterAuthProxyState = {
  userId: string;
  authUserId: string;
  /** Établissement du hostname courant (rôles scopés). */
  etablissementId: string | null;
  /** Établissement « maison » sur la ligne user (legacy / primaire). */
  homeEtablissementId: string | null;
  roles: string[];
  publicMetadata: Record<string, unknown>;
  orgAdmin: boolean;
  platformAdmin: boolean;
  mustChangePassword: boolean;
  twoFactorEnabled: boolean;
  requiresTwoFactorSetup: boolean;
};

export async function resolveBetterAuthProxyState(
  request: NextRequest,
  tenant?: TenantConfig,
): Promise<BetterAuthProxyState | null> {
  if (!isDatabaseConfigured()) return null;
  try {
    const session = await getBetterAuth().api.getSession({ headers: request.headers });
    if (!session?.user) return null;

    const u = session.user as typeof session.user & {
      etablissementId?: string;
      externalUserId?: string | null;
      orgAdmin?: boolean;
      platformAdmin?: boolean;
      mustChangePassword?: boolean;
      twoFactorEnabled?: boolean;
    };

    const db = getDb();
    const [row] = await db.select().from(user).where(eq(user.id, u.id)).limit(1);
    const homeEtablissementId = row?.etablissementId ?? u.etablissementId ?? null;
    const platformAdmin = Boolean(row?.platformAdmin ?? u.platformAdmin);

    let activeEtablissementId = homeEtablissementId;
    if (tenant && !platformAdmin && !isPlatformTenantSlug(tenant.slug)) {
      activeEtablissementId = await ensureEtablissementFromTenant(tenant);
    }

    const roles = activeEtablissementId
      ? await listUserRolesFromDb(u.id, activeEtablissementId)
      : [];
    // Aligné sur useIsOrgAdmin / isOrgAdminFromAppUser : flag DB OU rôle admin.
    const orgAdmin =
      Boolean(row?.orgAdmin ?? u.orgAdmin) || platformAdmin || roles.includes("admin");
    const businessUserId = row?.externalUserId?.trim() || u.id;
    const mustChangePassword = Boolean(row?.mustChangePassword ?? u.mustChangePassword);
    const twoFactorEnabled = Boolean(row?.twoFactorEnabled ?? u.twoFactorEnabled);
    const requiresTwoFactorSetup =
      roleRequiresTwoFactor({ platformAdmin, orgAdmin, roles }) && !twoFactorEnabled;

    return {
      userId: businessUserId,
      authUserId: u.id,
      etablissementId: activeEtablissementId,
      homeEtablissementId,
      roles,
      publicMetadata: {
        role: roles,
        org_admin: orgAdmin,
        platform_admin: platformAdmin,
        must_change_password: mustChangePassword,
        two_factor_enabled: twoFactorEnabled,
      },
      orgAdmin,
      platformAdmin,
      mustChangePassword,
      twoFactorEnabled,
      requiresTwoFactorSetup,
    };
  } catch (error) {
    console.error("[resolveBetterAuthProxyState]", error);
    return null;
  }
}

export async function resolveBetterAuthProxyStateByUserId(
  userId: string,
  etablissementId: string,
): Promise<BetterAuthProxyState | null> {
  if (!isDatabaseConfigured()) return null;
  const db = getDb();
  const [row] = await db
    .select()
    .from(user)
    .where(and(eq(user.id, userId), eq(user.etablissementId, etablissementId)))
    .limit(1);
  if (!row) return null;
  const roles = await listUserRolesFromDb(row.id, etablissementId);
  const orgAdmin =
    Boolean(row.orgAdmin) || Boolean(row.platformAdmin) || roles.includes("admin");
  const twoFactorEnabled = Boolean(row.twoFactorEnabled);
  const requiresTwoFactorSetup =
    roleRequiresTwoFactor({
      platformAdmin: row.platformAdmin,
      orgAdmin,
      roles,
    }) && !twoFactorEnabled;
  return {
    userId: row.externalUserId?.trim() || row.id,
    authUserId: row.id,
    etablissementId: row.etablissementId,
    homeEtablissementId: row.etablissementId,
    roles,
    publicMetadata: {
      role: roles,
      org_admin: orgAdmin,
      platform_admin: row.platformAdmin,
      must_change_password: row.mustChangePassword,
      two_factor_enabled: twoFactorEnabled,
    },
    orgAdmin,
    platformAdmin: row.platformAdmin,
    mustChangePassword: row.mustChangePassword,
    twoFactorEnabled,
    requiresTwoFactorSetup,
  };
}

/** Chemins autorisés tant que mustChangePassword est actif. */
export function isMustChangePasswordAllowedPath(pathname: string): boolean {
  const allow = [
    "/auth/change-password-required",
    "/auth/setup-2fa",
    "/auth/sign-out",
    "/sign-out",
    "/api/account/security",
    "/api/account/security-event",
    "/api/account/confirm-email",
    "/api/auth",
    "/api/auth/me",
    "/api/auth/status",
    "/api/auth/memberships",
    "/api/famille",
    "/famille",
  ];
  return allow.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}

/** Chemins autorisés tant que la 2FA obligatoire n’est pas configurée. */
export function isTwoFactorSetupAllowedPath(pathname: string): boolean {
  const allow = [
    "/auth/setup-2fa",
    "/auth/change-password-required",
    "/auth/sign-out",
    "/sign-out",
    "/api/account/security",
    "/api/account/security-event",
    "/api/auth",
    "/api/auth/me",
    "/api/auth/status",
    "/api/auth/memberships",
    "/api/famille",
    "/famille",
  ];
  return allow.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}
