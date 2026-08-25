import "server-only";

import { and, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { account, session, user } from "@/db/schema";
import { betterAuthBaseUrl } from "@/app/lib/auth-config";
import { getBetterAuth } from "@/app/lib/auth-server";

export type PasswordActivationTarget = {
  id: string;
  email: string;
  firstName: string | null;
  lastName: string | null;
  etablissementId: string;
  twoFactorEnabled: boolean;
};

export type PasswordActivationResult = {
  email: string;
  ok: boolean;
  skipped?: "mfa_already_enabled" | "not_found";
  detail?: string;
};

export async function listPasswordActivationTargets(opts?: {
  etablissementId?: string;
  email?: string;
}): Promise<PasswordActivationTarget[]> {
  const conditions = [eq(user.twoFactorEnabled, false)];
  const etablissementId = opts?.etablissementId?.trim();
  const email = opts?.email?.trim().toLowerCase();

  if (etablissementId) {
    conditions.push(eq(user.etablissementId, etablissementId));
  }
  if (email) {
    conditions.push(sql`lower(${user.email}) = ${email}`);
  }

  return getDb()
    .select({
      id: user.id,
      email: user.email,
      firstName: user.firstName,
      lastName: user.lastName,
      etablissementId: user.etablissementId,
      twoFactorEnabled: user.twoFactorEnabled,
    })
    .from(user)
    .where(and(...conditions))
    .orderBy(user.email);
}

export async function sendPasswordActivationToUser(
  target: PasswordActivationTarget,
): Promise<PasswordActivationResult> {
  if (target.twoFactorEnabled) {
    return { email: target.email, ok: false, skipped: "mfa_already_enabled" };
  }

  const db = getDb();
  const auth = getBetterAuth();
  const redirectTo = `${betterAuthBaseUrl()}/auth/reset-password`;

  try {
    await db.delete(session).where(eq(session.userId, target.id));
    await db
      .delete(account)
      .where(and(eq(account.userId, target.id), eq(account.providerId, "credential")));
    await db
      .update(user)
      .set({
        mustChangePassword: true,
        updatedAt: new Date(),
      })
      .where(eq(user.id, target.id));

    await auth.api.requestPasswordReset({
      body: {
        email: target.email,
        redirectTo,
      },
    });

    return { email: target.email, ok: true };
  } catch (error) {
    return {
      email: target.email,
      ok: false,
      detail: error instanceof Error ? error.message : String(error),
    };
  }
}

export async function sendPasswordActivationBatch(opts?: {
  etablissementId?: string;
  email?: string;
  delayMs?: number;
}): Promise<{
  baseUrl: string;
  redirectTo: string;
  results: PasswordActivationResult[];
}> {
  const delayMs = Math.max(0, opts?.delayMs ?? 400);
  const targets = await listPasswordActivationTargets(opts);
  const redirectTo = `${betterAuthBaseUrl()}/auth/reset-password`;
  const results: PasswordActivationResult[] = [];

  for (const target of targets) {
    results.push(await sendPasswordActivationToUser(target));
    if (delayMs > 0) {
      await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
  }

  return {
    baseUrl: betterAuthBaseUrl(),
    redirectTo,
    results,
  };
}
