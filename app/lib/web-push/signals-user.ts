import "server-only";

import { and, eq } from "drizzle-orm";
import { getDb } from "@/db/index";
import { user } from "@/db/schema";
import {
  isOrgAdminFromAppUser,
  isPlatformMasterFromAppUser,
  listUserRolesFromDb,
  resolveBusinessUserId,
} from "@/app/lib/auth-roles-db";
import type { AppUser } from "@/app/lib/app-session";

/** Personnel = au moins un rôle autre que parent/élève (aligné sur BrowserNotificationsProvider). */
function hasStaffRole(roles: string[], orgAdmin: boolean, platformAdmin: boolean): boolean {
  if (orgAdmin || platformAdmin) return true;
  const visible = roles.filter((r) => r !== "master");
  if (visible.length === 0) return roles.includes("master");
  return visible.some((r) => r !== "parent" && r !== "eleve");
}

/**
 * Reconstruit l'AppUser hors requête HTTP (même champs que betterAuthSessionToAppUser, sans cookies).
 * Retourne null si l'utilisateur est introuvable, hors établissement ou sans rôle de personnel.
 */
export async function loadAppUserById(
  authUserId: string,
  etablissementId: string,
): Promise<AppUser | null> {
  const [u] = await getDb()
    .select()
    .from(user)
    .where(and(eq(user.id, authUserId), eq(user.etablissementId, etablissementId)))
    .limit(1);
  if (!u) return null;
  const roles = await listUserRolesFromDb(u.id, etablissementId);
  if (roles.length === 0) return null;
  const orgAdmin = Boolean(u.orgAdmin) || isOrgAdminFromAppUser({ roles, orgAdmin: u.orgAdmin });
  const platformAdmin =
    Boolean(u.platformAdmin) || isPlatformMasterFromAppUser({ roles, platformAdmin: u.platformAdmin });
  if (!hasStaffRole(roles, orgAdmin, platformAdmin)) return null;
  const businessUserId =
    u.externalUserId?.trim() || (await resolveBusinessUserId(u.id, etablissementId));
  return {
    id: u.id,
    businessUserId,
    email: u.email,
    firstName: u.firstName ?? undefined,
    lastName: u.lastName ?? undefined,
    name: u.name,
    imageUrl: u.image ?? undefined,
    etablissementId,
    roles,
    orgAdmin,
    platformAdmin,
    twoFactorEnabled: Boolean(u.twoFactorEnabled),
    hasPasskey: false,
    mfaSatisfied: true,
    externalUserId: u.externalUserId ?? undefined,
    authSource: "better-auth",
  };
}
