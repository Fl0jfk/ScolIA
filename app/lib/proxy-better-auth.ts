import "server-only";

import type { NextRequest } from "next/server";
import { and, eq } from "drizzle-orm";
import { isBetterAuthActive, isBetterAuthPilotPath } from "@/app/lib/auth-config";
import { getBetterAuth } from "@/app/lib/auth-server";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user } from "@/db/schema";

export type BetterAuthProxyState = {
  userId: string;
  authUserId: string;
  etablissementId: string | null;
  roles: string[];
  publicMetadata: Record<string, unknown>;
  orgAdmin: boolean;
  platformAdmin: boolean;
  mustChangePassword: boolean;
};

export function proxyUsesBetterAuth(pathname: string): boolean {
  return isBetterAuthActive() && isBetterAuthPilotPath(pathname);
}

export async function resolveBetterAuthProxyState(
  request: NextRequest,
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
    };

    const db = getDb();
    const [row] = await db.select().from(user).where(eq(user.id, u.id)).limit(1);
    const etablissementId = row?.etablissementId ?? u.etablissementId ?? null;
    const roles = etablissementId ? await listUserRolesFromDb(u.id, etablissementId) : [];
    const businessUserId = row?.externalUserId?.trim() || u.id;
    const orgAdmin = Boolean(row?.orgAdmin ?? u.orgAdmin);
    const platformAdmin = Boolean(row?.platformAdmin ?? u.platformAdmin);
    const mustChangePassword = Boolean(row?.mustChangePassword ?? u.mustChangePassword);

    return {
      userId: businessUserId,
      authUserId: u.id,
      etablissementId,
      roles,
      publicMetadata: {
        role: roles,
        org_admin: orgAdmin,
        platform_admin: platformAdmin,
        must_change_password: mustChangePassword,
      },
      orgAdmin,
      platformAdmin,
      mustChangePassword,
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
  return {
    userId: row.externalUserId?.trim() || row.id,
    authUserId: row.id,
    etablissementId: row.etablissementId,
    roles,
    publicMetadata: {
      role: roles,
      org_admin: row.orgAdmin,
      platform_admin: row.platformAdmin,
      must_change_password: row.mustChangePassword,
    },
    orgAdmin: row.orgAdmin,
    platformAdmin: row.platformAdmin,
    mustChangePassword: row.mustChangePassword,
  };
}

/** Chemins autorisés tant que mustChangePassword est actif. */
export function isMustChangePasswordAllowedPath(pathname: string): boolean {
  const allow = [
    "/auth/change-password-required",
    "/auth/sign-out",
    "/sign-out",
    "/api/account/security",
    "/api/account/confirm-email",
    "/api/auth",
    "/api/auth/me",
    "/api/auth/status",
  ];
  return allow.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}
