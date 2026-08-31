import "server-only";

import { headers } from "next/headers";
import { isBetterAuthActive } from "@/app/lib/auth-config";
import { getBetterAuth } from "@/app/lib/auth-server";
import {
  isOrgAdminFromAppUser,
  isPlatformMasterFromAppUser,
  listUserRolesFromDb,
  resolveBusinessUserId,
} from "@/app/lib/auth-roles-db";
import { normalizeIntranetRoles } from "@/app/lib/intranet-roles";

export type AuthSource = "better-auth";

export type AppUser = {
  id: string;
  businessUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  name?: string;
  imageUrl?: string;
  etablissementId?: string;
  roles: string[];
  orgAdmin: boolean;
  platformAdmin: boolean;
  twoFactorEnabled: boolean;
  externalUserId?: string;
  authSource: AuthSource;
};

export type AppSession = {
  user: AppUser;
};

/** Forme minimale compatible avec l’ancien profil session (publicMetadata). */
export type CompatAuthUser = {
  id: string;
  firstName: string | null;
  lastName: string | null;
  fullName: string | null;
  imageUrl: string;
  primaryEmailAddressId: string | null;
  primaryEmailAddress: { emailAddress: string } | null;
  emailAddresses: { id: string; emailAddress: string }[];
  publicMetadata: Record<string, unknown>;
};

async function betterAuthSessionToAppUser(): Promise<AppUser | null> {
  if (!isBetterAuthActive()) return null;
  try {
    const hdrs = await headers();
    const session = await getBetterAuth().api.getSession({ headers: hdrs });
    if (!session?.user) return null;

    const u = session.user as typeof session.user & {
      etablissementId?: string;
      externalUserId?: string | null;
      firstName?: string | null;
      lastName?: string | null;
      orgAdmin?: boolean;
      platformAdmin?: boolean;
      twoFactorEnabled?: boolean;
    };

    const etablissementId = u.etablissementId;
    const roles = etablissementId
      ? await listUserRolesFromDb(u.id, etablissementId)
      : normalizeIntranetRoles([]);
    const businessUserId =
      u.externalUserId?.trim() ||
      (etablissementId ? await resolveBusinessUserId(u.id, etablissementId) : u.id);

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
      orgAdmin: Boolean(u.orgAdmin) || isOrgAdminFromAppUser({ roles, orgAdmin: u.orgAdmin }),
      platformAdmin:
        Boolean(u.platformAdmin) ||
        isPlatformMasterFromAppUser({ roles, platformAdmin: u.platformAdmin }),
      twoFactorEnabled: Boolean(u.twoFactorEnabled),
      externalUserId: u.externalUserId ?? undefined,
      authSource: "better-auth",
    };
  } catch (error) {
    console.error("[getAppSession:better-auth]", error);
    return null;
  }
}

/** Session applicative (Better-Auth). */
export async function getAppSession(): Promise<AppSession | null> {
  const betterAuthUser = await betterAuthSessionToAppUser();
  return betterAuthUser ? { user: betterAuthUser } : null;
}

export async function resolveAppSessionIds(): Promise<{ userId: string } | null> {
  const session = await getAppSession();
  if (!session) return null;
  return { userId: session.user.businessUserId };
}

export async function requireAppUser(): Promise<
  { ok: true; user: AppUser } | { ok: false; reason: "unauthorized" | "unavailable" }
> {
  try {
    const session = await getAppSession();
    if (!session) return { ok: false, reason: "unauthorized" };
    return { ok: true, user: session.user };
  } catch (error) {
    console.error("[requireAppUser]", error);
    return { ok: false, reason: "unavailable" };
  }
}

export async function resolveSession(): Promise<{ userId: string } | null> {
  return resolveAppSessionIds();
}

export async function safeCurrentUser(): Promise<CompatAuthUser | null> {
  const session = await getAppSession();
  if (!session) return null;
  const u = session.user;
  const fullName =
    u.name?.trim() ||
    `${u.firstName ?? ""} ${u.lastName ?? ""}`.trim() ||
    u.email ||
    null;
  return {
    id: u.businessUserId,
    firstName: u.firstName ?? null,
    lastName: u.lastName ?? null,
    fullName,
    imageUrl: u.imageUrl ?? "",
    primaryEmailAddressId: "primary",
    primaryEmailAddress: u.email ? { emailAddress: u.email } : null,
    emailAddresses: u.email ? [{ id: "primary", emailAddress: u.email }] : [],
    publicMetadata: {
      role: u.roles,
      org_admin: u.orgAdmin,
      platform_admin: u.platformAdmin,
    },
  };
}
