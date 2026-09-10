import "server-only";

import type { NextRequest } from "next/server";
import { and, eq } from "drizzle-orm";
import { isBetterAuthActive } from "@/app/lib/auth-config";
import { getBetterAuth } from "@/app/lib/auth-server";
import { listUserRolesFromDb } from "@/app/lib/auth-roles-db";
import { ensureEtablissementFromTenant } from "@/app/lib/etablissement-db";
import { isPlatformTenantSlug } from "@/app/lib/platform-tenant";
import { roleRequiresTwoFactor, isMfaSatisfied } from "@/app/lib/two-factor-policy";
import { checkUserHasPasskey } from "@/app/lib/passkey-db";
import type { TenantConfig } from "@/app/lib/tenant-types";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { user } from "@/db/schema";
import {
  cacheGetProxyAuth,
  cacheSetProxyAuth,
} from "@/app/lib/valkey-cache";
import { createHash } from "node:crypto";

export type BetterAuthProxyState = {
  userId: string;
  authUserId: string;
  email: string;
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
  hasPasskey: boolean;
  requiresTwoFactorSetup: boolean;
};

/** L1 process — évite même le RTT Valkey / getSession sur la même instance. */
const PROXY_L1_TTL_MS = 45_000;
const proxyL1 = new Map<string, { state: BetterAuthProxyState; expiresAt: number }>();

function proxyL1Key(userId: string, scope: string): string {
  return `${userId}:${scope}`;
}

function sessionTokenFromRequest(request: NextRequest): string | null {
  return (
    request.cookies.get("__Secure-better-auth.session_token")?.value?.trim() ||
    request.cookies.get("better-auth.session_token")?.value?.trim() ||
    null
  );
}

function tokenL1Key(token: string, scope: string): string {
  const hash = createHash("sha256").update(token).digest("hex").slice(0, 32);
  return `tok:${scope}:${hash}`;
}

function readL1(key: string): BetterAuthProxyState | null {
  const hit = proxyL1.get(key);
  if (!hit || hit.expiresAt <= Date.now()) {
    if (hit) proxyL1.delete(key);
    return null;
  }
  return hit.state;
}

function writeL1(keys: string[], state: BetterAuthProxyState): void {
  const entry = { state, expiresAt: Date.now() + PROXY_L1_TTL_MS };
  for (const k of keys) proxyL1.set(k, entry);
}

export async function resolveBetterAuthProxyState(
  request: NextRequest,
  tenant?: TenantConfig,
): Promise<BetterAuthProxyState | null> {
  if (!isDatabaseConfigured()) return null;
  try {
    const cacheScope = tenant?.slug?.trim() || "_";
    const sessionToken = sessionTokenFromRequest(request);
    if (sessionToken) {
      const fromToken = readL1(tokenL1Key(sessionToken, cacheScope));
      if (fromToken) return fromToken;
    }

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

    const scope = tenant?.slug?.trim() || u.etablissementId || "_";
    const l1k = proxyL1Key(u.id, scope);
    const fromUser = readL1(l1k);
    if (fromUser) {
      if (sessionToken) writeL1([tokenL1Key(sessionToken, scope)], fromUser);
      return fromUser;
    }

    const cached = await cacheGetProxyAuth<BetterAuthProxyState>(u.id, scope);
    if (cached?.authUserId === u.id) {
      const keys = [l1k];
      if (sessionToken) keys.push(tokenL1Key(sessionToken, scope));
      writeL1(keys, cached);
      return cached;
    }

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
    const mfaRequired = roleRequiresTwoFactor({ platformAdmin, orgAdmin, roles });
    /**
     * Évite un round-trip Scaleway quand MFA déjà OK (TOTP) ou non exigée.
     * Cache Valkey + mémoire pour les navigations suivantes.
     */
    let hasPasskey = false;
    let passkeyCheckFailed = false;
    if (mfaRequired && !twoFactorEnabled) {
      const passkeyStatus = await checkUserHasPasskey(u.id);
      hasPasskey = passkeyStatus.hasPasskey;
      passkeyCheckFailed = passkeyStatus.checkFailed;
    } else if (twoFactorEnabled) {
      // MFA déjà satisfaite via TOTP — pas besoin de requêter passkey pour le gate.
      hasPasskey = false;
    }
    const mfaSatisfied = isMfaSatisfied({ twoFactorEnabled, hasPasskey });
    const requiresTwoFactorSetup =
      mfaRequired && !mfaSatisfied && !passkeyCheckFailed;

    const state: BetterAuthProxyState = {
      userId: businessUserId,
      authUserId: u.id,
      email: String(u.email || row?.email || "").trim(),
      etablissementId: activeEtablissementId,
      homeEtablissementId,
      roles,
      publicMetadata: {
        role: roles,
        org_admin: orgAdmin,
        platform_admin: platformAdmin,
        must_change_password: mustChangePassword,
        two_factor_enabled: twoFactorEnabled,
        has_passkey: hasPasskey,
        mfa_satisfied: mfaSatisfied,
      },
      orgAdmin,
      platformAdmin,
      mustChangePassword,
      twoFactorEnabled,
      hasPasskey,
      requiresTwoFactorSetup,
    };
    void cacheSetProxyAuth(u.id, scope, state);
    const keys = [proxyL1Key(u.id, scope)];
    if (sessionToken) keys.push(tokenL1Key(sessionToken, scope));
    writeL1(keys, state);
    return state;
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
  const mfaRequired = roleRequiresTwoFactor({
    platformAdmin: row.platformAdmin,
    orgAdmin,
    roles,
  });
  let hasPasskey = false;
  let passkeyCheckFailed = false;
  if (mfaRequired && !twoFactorEnabled) {
    const passkeyStatus = await checkUserHasPasskey(row.id);
    hasPasskey = passkeyStatus.hasPasskey;
    passkeyCheckFailed = passkeyStatus.checkFailed;
  }
  const mfaSatisfied = isMfaSatisfied({ twoFactorEnabled, hasPasskey });
  const requiresTwoFactorSetup = mfaRequired && !mfaSatisfied && !passkeyCheckFailed;
  return {
    userId: row.externalUserId?.trim() || row.id,
    authUserId: row.id,
    email: String(row.email || "").trim(),
    etablissementId: row.etablissementId,
    homeEtablissementId: row.etablissementId,
    roles,
    publicMetadata: {
      role: roles,
      org_admin: orgAdmin,
      platform_admin: row.platformAdmin,
      must_change_password: row.mustChangePassword,
      two_factor_enabled: twoFactorEnabled,
      has_passkey: hasPasskey,
      mfa_satisfied: mfaSatisfied,
    },
    orgAdmin,
    platformAdmin: row.platformAdmin,
    mustChangePassword: row.mustChangePassword,
    twoFactorEnabled,
    hasPasskey,
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
    "/api/account/two-factor",
    "/api/account/passkeys",
    "/api/account/sessions",
    "/api/auth",
    "/api/auth/me",
    "/api/auth/status",
    "/api/auth/memberships",
    "/api/famille",
    "/famille",
    "/api/eleve",
    "/api/mobile",
    "/app-mobile",
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
    "/api/account/two-factor",
    "/api/account/passkeys",
    "/api/account/sessions",
    "/api/auth",
    "/api/auth/me",
    "/api/auth/status",
    "/api/auth/memberships",
    "/api/famille",
    "/famille",
    "/api/eleve",
    "/api/mobile",
    "/app-mobile",
  ];
  return allow.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}
