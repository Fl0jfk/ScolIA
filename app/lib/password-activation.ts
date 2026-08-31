import "server-only";

import { and, eq, sql } from "drizzle-orm";
import { getDb } from "@/db/index";
import { account, session, twoFactor, user } from "@/db/schema";
import { betterAuthBaseUrl } from "@/app/lib/auth-config";
import { getBetterAuth } from "@/app/lib/auth-server";
import { getPlatformSmtpConfig } from "@/app/lib/tenant-mail";
import { listUserRolesBatchFromDb } from "@/app/lib/auth-roles-db";
import { isAccountActivationPending } from "@/app/lib/two-factor-policy";
import { ensureUserInvitationSentAtColumn, INVITATION_RECENT_MS } from "@/app/lib/user-invitation-sent";

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
  resetMfa?: boolean;
  skipped?: "not_found" | "smtp_unavailable";
  detail?: string;
};

export async function listPasswordActivationTargets(opts?: {
  etablissementId?: string;
  email?: string;
  /** Si true, inclut aussi les comptes déjà en MFA (reset complet). */
  includeMfa?: boolean;
}): Promise<PasswordActivationTarget[]> {
  const conditions = opts?.includeMfa ? [] : [eq(user.twoFactorEnabled, false)];
  const etablissementId = opts?.etablissementId?.trim();
  const email = opts?.email?.trim().toLowerCase();

  if (etablissementId) {
    conditions.push(eq(user.etablissementId, etablissementId));
  }
  if (email) {
    conditions.push(sql`lower(${user.email}) = ${email}`);
  }

  const where = conditions.length ? and(...conditions) : undefined;

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
    .where(where)
    .orderBy(user.email);
}

export async function listPendingInvitationTargets(opts: {
  etablissementId: string;
}): Promise<PasswordActivationTarget[]> {
  await ensureUserInvitationSentAtColumn();
  const cutoff = new Date(Date.now() - INVITATION_RECENT_MS);
  const rows = await getDb()
    .select({
      id: user.id,
      email: user.email,
      firstName: user.firstName,
      lastName: user.lastName,
      etablissementId: user.etablissementId,
      twoFactorEnabled: user.twoFactorEnabled,
      invitationSentAt: user.invitationSentAt,
      emailVerified: user.emailVerified,
      mustChangePassword: user.mustChangePassword,
      platformAdmin: user.platformAdmin,
      orgAdmin: user.orgAdmin,
    })
    .from(user)
    .where(eq(user.etablissementId, opts.etablissementId))
    .orderBy(user.email);

  const rolesByUserId = await listUserRolesBatchFromDb(
    rows.map((r) => r.id),
    opts.etablissementId,
  );

  return rows
    .filter((r) => {
      if (r.invitationSentAt && r.invitationSentAt >= cutoff) return false;
      const roles = rolesByUserId.get(r.id) ?? [];
      return isAccountActivationPending({
        emailVerified: r.emailVerified,
        mustChangePassword: r.mustChangePassword,
        twoFactorEnabled: r.twoFactorEnabled,
        platformAdmin: r.platformAdmin,
        orgAdmin: r.orgAdmin || roles.includes("admin"),
        roles,
      });
    })
    .map(
      ({
        invitationSentAt: _invitationSentAt,
        emailVerified: _emailVerified,
        mustChangePassword: _mustChangePassword,
        platformAdmin: _platformAdmin,
        orgAdmin: _orgAdmin,
        ...target
      }) => target,
    );
}

/** Statut d’un compte pour l’envoi d’un lien d’invitation. */
export async function resolveInvitationTarget(opts: {
  email: string;
  etablissementId: string;
}): Promise<{ status: "ready"; target: PasswordActivationTarget } | { status: "not_found" }> {
  const email = opts.email.trim().toLowerCase();
  const [row] = await getDb()
    .select({
      id: user.id,
      email: user.email,
      firstName: user.firstName,
      lastName: user.lastName,
      etablissementId: user.etablissementId,
      twoFactorEnabled: user.twoFactorEnabled,
    })
    .from(user)
    .where(and(eq(user.etablissementId, opts.etablissementId), sql`lower(${user.email}) = ${email}`))
    .limit(1);

  if (!row) return { status: "not_found" };
  return { status: "ready", target: row };
}

/**
 * Invalide MDP + sessions, et optionnellement la MFA (repart de zéro).
 * Puis envoie le mail de lien d’invitation / reset.
 */
export async function sendPasswordActivationToUser(
  target: PasswordActivationTarget,
  opts?: { resetMfa?: boolean },
): Promise<PasswordActivationResult> {
  const resetMfa = opts?.resetMfa === true || target.twoFactorEnabled;

  if (!getPlatformSmtpConfig()) {
    return {
      email: target.email,
      ok: false,
      skipped: "smtp_unavailable",
      detail: "Envoi d’e-mail indisponible (SMTP non configuré sur cet environnement).",
    };
  }

  const db = getDb();
  const auth = getBetterAuth();
  const redirectTo = `${betterAuthBaseUrl()}/auth/reset-password`;

  try {
    await ensureUserInvitationSentAtColumn();
    await db.delete(session).where(eq(session.userId, target.id));
    await db
      .delete(account)
      .where(and(eq(account.userId, target.id), eq(account.providerId, "credential")));

    // Toujours purger un setup MFA incomplet (QR généré puis abandonné).
    // resetMfa = true : purge aussi une MFA déjà finalisée.
    if (resetMfa || !target.twoFactorEnabled) {
      await db.delete(twoFactor).where(eq(twoFactor.userId, target.id));
    }

    const now = new Date();
    await db
      .update(user)
      .set({
        mustChangePassword: true,
        ...(resetMfa ? { twoFactorEnabled: false } : {}),
        invitationSentAt: now,
        updatedAt: now,
      })
      .where(eq(user.id, target.id));

    await auth.api.requestPasswordReset({
      body: {
        email: target.email,
        redirectTo,
      },
    });

    return { email: target.email, ok: true, resetMfa };
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
  /** Bulk : par défaut on n’inclut pas les comptes MFA déjà activés. */
  includeMfa?: boolean;
}): Promise<{
  baseUrl: string;
  redirectTo: string;
  results: PasswordActivationResult[];
}> {
  const delayMs = Math.max(0, opts?.delayMs ?? 400);
  const targets = await listPasswordActivationTargets({
    etablissementId: opts?.etablissementId,
    email: opts?.email,
    includeMfa: opts?.includeMfa,
  });
  const redirectTo = `${betterAuthBaseUrl()}/auth/reset-password`;
  const results: PasswordActivationResult[] = [];

  for (const target of targets) {
    results.push(await sendPasswordActivationToUser(target, { resetMfa: opts?.includeMfa === true }));
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
