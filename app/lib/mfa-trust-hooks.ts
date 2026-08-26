import "server-only";

import { expireCookie } from "better-auth/cookies";
import {
  resolveMfaTrustPolicyForEmail,
  resolveMfaTrustPolicyForUserId,
} from "@/app/lib/mfa-trust-resolve";
import type { MfaTrustPolicy } from "@/app/lib/two-factor-policy";

const TRUST_DEVICE_COOKIE_NAME = "trust_device";

type AuthHookCtx = {
  path: string;
  body?: unknown;
  context: {
    secret: string;
    newSession?: { user: { id: string }; session: { token: string } } | null;
    createAuthCookie: (
      name: string,
      opts?: { maxAge?: number },
    ) => { name: string; attributes: Record<string, unknown> };
    internalAdapter: {
      deleteVerificationByIdentifier: (id: string) => Promise<unknown>;
      createVerificationValue: (opts: {
        value: string;
        identifier: string;
        expiresAt: Date;
      }) => Promise<unknown>;
      findVerificationValue: (
        id: string,
      ) => Promise<{ value: string; expiresAt: Date } | null>;
    };
  };
  getSignedCookie: (name: string, secret: string) => Promise<string | false>;
  setSignedCookie: (
    name: string,
    value: string,
    secret: string,
    attributes: Record<string, unknown>,
  ) => Promise<unknown>;
};

function trustCookieAttrs(ctx: AuthHookCtx, maxAge: number) {
  return ctx.context.createAuthCookie(TRUST_DEVICE_COOKIE_NAME, {
    maxAge: Math.max(1, maxAge),
  });
}

async function clearTrustDeviceCookie(ctx: AuthHookCtx): Promise<void> {
  const cookie = trustCookieAttrs(ctx, 1);
  const value = await ctx.getSignedCookie(cookie.name, ctx.context.secret);
  if (value) {
    const parts = value.split("!");
    const trustIdentifier = parts[1];
    if (trustIdentifier) {
      try {
        await ctx.context.internalAdapter.deleteVerificationByIdentifier(trustIdentifier);
      } catch {
        /* ignore */
      }
    }
  }
  expireCookie(ctx as never, cookie as never);
}

async function clampTrustDeviceCookie(
  ctx: AuthHookCtx,
  userId: string,
  policy: MfaTrustPolicy,
): Promise<void> {
  if (!policy.allowTrust || policy.maxAgeSeconds <= 0) {
    await clearTrustDeviceCookie(ctx);
    return;
  }

  const cookie = trustCookieAttrs(ctx, policy.maxAgeSeconds);
  const value = await ctx.getSignedCookie(cookie.name, ctx.context.secret);
  if (!value) return;

  const parts = value.split("!");
  const trustIdentifier = parts[1];
  if (!trustIdentifier) return;

  try {
    await ctx.context.internalAdapter.deleteVerificationByIdentifier(trustIdentifier);
  } catch {
    /* ignore */
  }
  await ctx.context.internalAdapter.createVerificationValue({
    value: userId,
    identifier: trustIdentifier,
    expiresAt: new Date(Date.now() + policy.maxAgeSeconds * 1000),
  });
  await ctx.setSignedCookie(cookie.name, value, ctx.context.secret, cookie.attributes);
}

/** Avant login e-mail : admin → invalide tout trust device (MFA obligatoire). */
export async function mfaTrustBeforeSignIn(ctx: AuthHookCtx): Promise<void> {
  if (ctx.path !== "/sign-in/email") return;
  const body = ctx.body as { email?: string } | undefined;
  const email = typeof body?.email === "string" ? body.email : "";
  if (!email) return;
  const policy = await resolveMfaTrustPolicyForEmail(email);
  if (!policy || policy.allowTrust) return;
  await clearTrustDeviceCookie(ctx);
}

/**
 * Après verify TOTP / backup : applique la durée réelle (0 / 7j / 30j).
 * Better-Auth pose toujours 30j si trustDevice=true — on recale ensuite.
 */
export async function mfaTrustAfterVerify(ctx: AuthHookCtx): Promise<void> {
  if (
    ctx.path !== "/two-factor/verify-totp" &&
    ctx.path !== "/two-factor/verify-backup-code"
  ) {
    return;
  }
  const session = ctx.context.newSession;
  const userId = session?.user?.id;
  if (!userId) return;
  const policy = await resolveMfaTrustPolicyForUserId(userId);
  if (!policy) return;

  const body = ctx.body as { trustDevice?: boolean } | undefined;
  if (!body?.trustDevice && policy.allowTrust) return;

  await clampTrustDeviceCookie(ctx, userId, policy);
}

/**
 * Après login si l’appareil était déjà trusted : Better-Auth refresh à 30j.
 * On re-cadre direction à 7j (admin déjà bloqué en before).
 */
export async function mfaTrustAfterSignIn(ctx: AuthHookCtx): Promise<void> {
  if (
    ctx.path !== "/sign-in/email" &&
    ctx.path !== "/sign-in/username" &&
    ctx.path !== "/sign-in/phone-number"
  ) {
    return;
  }
  const session = ctx.context.newSession;
  const userId = session?.user?.id;
  if (!userId) return; // challenge 2FA en cours ou échec
  const policy = await resolveMfaTrustPolicyForUserId(userId);
  if (!policy) return;
  if (!policy.allowTrust) {
    await clearTrustDeviceCookie(ctx);
    return;
  }
  if (policy.maxAgeSeconds < 60 * 60 * 24 * 30) {
    await clampTrustDeviceCookie(ctx, userId, policy);
  }
}
