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
  roles: string[];
  publicMetadata: Record<string, unknown>;
  orgAdmin: boolean;
  platformAdmin: boolean;
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
    };

    const db = getDb();
    const [row] = await db.select().from(user).where(eq(user.id, u.id)).limit(1);
    const etablissementId = row?.etablissementId ?? u.etablissementId;
    const roles = etablissementId ? await listUserRolesFromDb(u.id, etablissementId) : [];
    const businessUserId = row?.externalUserId?.trim() || u.id;
    const orgAdmin = Boolean(row?.orgAdmin ?? u.orgAdmin);
    const platformAdmin = Boolean(row?.platformAdmin ?? u.platformAdmin);

    return {
      userId: businessUserId,
      roles,
      publicMetadata: {
        role: roles,
        org_admin: orgAdmin,
        platform_admin: platformAdmin,
      },
      orgAdmin,
      platformAdmin,
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
    roles,
    publicMetadata: {
      role: roles,
      org_admin: row.orgAdmin,
      platform_admin: row.platformAdmin,
    },
    orgAdmin: row.orgAdmin,
    platformAdmin: row.platformAdmin,
  };
}
