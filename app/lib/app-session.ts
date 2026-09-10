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
import { userHasPasskey } from "@/app/lib/passkey-db";
import { isMfaSatisfied } from "@/app/lib/two-factor-policy";
import {
  decodeAuthSnapshot,
  SCOLA_AUTH_SNAPSHOT_HEADER,
} from "@/app/lib/auth-snapshot";

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
  hasPasskey: boolean;
  mfaSatisfied: boolean;
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

    // Snapshot posé par le proxy : évite un 2e getSession + SELECT rôles.
    const fromProxy = decodeAuthSnapshot(hdrs.get(SCOLA_AUTH_SNAPSHOT_HEADER));
    if (fromProxy) {
      const roles = normalizeIntranetRoles(fromProxy.roles);
      return {
        id: fromProxy.authUserId,
        businessUserId: fromProxy.userId,
        email: fromProxy.email,
        firstName: fromProxy.firstName,
        lastName: fromProxy.lastName,
        name: fromProxy.name,
        imageUrl: fromProxy.imageUrl,
        etablissementId: fromProxy.etablissementId ?? undefined,
        roles,
        orgAdmin:
          fromProxy.orgAdmin ||
          isOrgAdminFromAppUser({ roles, orgAdmin: fromProxy.orgAdmin }),
        platformAdmin:
          fromProxy.platformAdmin ||
          isPlatformMasterFromAppUser({
            roles,
            platformAdmin: fromProxy.platformAdmin,
          }),
        twoFactorEnabled: fromProxy.twoFactorEnabled,
        hasPasskey: fromProxy.hasPasskey,
        mfaSatisfied: isMfaSatisfied({
          twoFactorEnabled: fromProxy.twoFactorEnabled,
          hasPasskey: fromProxy.hasPasskey,
        }),
        externalUserId: fromProxy.externalUserId,
        authSource: "better-auth",
      };
    }

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
    const twoFactorEnabled = Boolean(u.twoFactorEnabled);
    // Skip round-trip Scaleway si TOTP déjà OK (cache 60s sinon via passkey-db).
    const hasPasskey = twoFactorEnabled ? false : await userHasPasskey(u.id);

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
      twoFactorEnabled,
      hasPasskey,
      mfaSatisfied: isMfaSatisfied({ twoFactorEnabled, hasPasskey }),
      externalUserId: u.externalUserId ?? undefined,
      authSource: "better-auth",
    };
  } catch (error) {
    console.error("[getAppSession:better-auth]", error);
    return null;
  }
}

/** Session applicative (Better-Auth) — toujours l’acteur réel authentifié. */
export async function getAppSession(): Promise<AppSession | null> {
  const betterAuthUser = await betterAuthSessionToAppUser();
  return betterAuthUser ? { user: betterAuthUser } : null;
}

/**
 * Utilisateur « vu » : cible de supervision lecture seule si active, sinon l’acteur.
 * À utiliser pour modules, menus, listes « mon espace ». Pas pour les gardes admin / audit acteur.
 */
export async function getEffectiveViewUser(): Promise<AppUser | null> {
  const session = await getAppSession();
  if (!session) return null;
  try {
    const { resolveActiveSupervision, supervisionTargetToAppUser } = await import(
      "@/app/lib/supervision"
    );
    const active = await resolveActiveSupervision({ actorUserId: session.user.id });
    if (active) return supervisionTargetToAppUser(active.target);
  } catch (error) {
    console.error("[getEffectiveViewUser]", error);
  }
  return session.user;
}

export async function resolveAppSessionIds(): Promise<{ userId: string } | null> {
  const user = await getEffectiveViewUser();
  if (!user) return null;
  return { userId: user.businessUserId };
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

/** Comme requireAppUser mais avec la vue supervision (cible) si active. */
export async function requireViewUser(): Promise<
  { ok: true; user: AppUser } | { ok: false; reason: "unauthorized" | "unavailable" }
> {
  try {
    const user = await getEffectiveViewUser();
    if (!user) return { ok: false, reason: "unauthorized" };
    return { ok: true, user };
  } catch (error) {
    console.error("[requireViewUser]", error);
    return { ok: false, reason: "unavailable" };
  }
}

export async function resolveSession(): Promise<{ userId: string } | null> {
  return resolveAppSessionIds();
}

function appUserToCompat(u: AppUser): CompatAuthUser {
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

/** Profil compat pour l’UI / signaux — vue effective (supervision). */
export async function safeCurrentUser(): Promise<CompatAuthUser | null> {
  const user = await getEffectiveViewUser();
  if (!user) return null;
  return appUserToCompat(user);
}
