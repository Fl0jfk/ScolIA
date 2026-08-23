import { NextResponse } from "next/server";
import { createHash, randomBytes } from "crypto";
import { and, eq, like, ne, sql } from "drizzle-orm";
import { hashPassword } from "better-auth/crypto";
import { getBetterAuth } from "@/app/lib/auth-server";
import { consumeRateLimit } from "@/app/lib/rate-limit";
import { validatePasswordPolicy } from "@/app/lib/password-policy";
import { writeSecurityAudit } from "@/app/lib/security-audit";
import { createPlatformTransporter } from "@/app/lib/tenant-mail";
import { getDb } from "@/db/index";
import { account, user, verification } from "@/db/schema";

type Body =
  | {
      action: "password";
      currentPassword: string;
      newPassword: string;
    }
  | {
      action: "email";
      currentPassword: string;
      newEmail: string;
    };

function isEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

function clientKey(req: Request, userId: string): string {
  const fwd = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() || "unknown";
  return `account-security:${userId}:${fwd}`;
}

async function sendSecurityMail(opts: {
  to: string;
  subject: string;
  text: string;
}): Promise<boolean> {
  const transporter = createPlatformTransporter();
  if (!transporter) return false;
  try {
    const from =
      process.env.MAILER_EMAIL?.trim() ||
      process.env.SMTP_USER?.trim() ||
      "mailer@scolia.fr";
    await transporter.sendMail({
      from: `"ScolIA" <${from}>`,
      to: opts.to,
      subject: opts.subject,
      text: opts.text,
    });
    return true;
  } catch (error) {
    console.error("[account/security] mail", error);
    return false;
  }
}

function requestOrigin(req: Request): string {
  const url = new URL(req.url);
  const proto = req.headers.get("x-forwarded-proto") || url.protocol.replace(":", "");
  const host =
    req.headers.get("x-forwarded-host") || req.headers.get("host") || url.host;
  return `${proto}://${host}`;
}

/** Mise à jour e-mail / mot de passe du compte connecté. */
export async function POST(req: Request) {
  let body: Body;
  try {
    body = (await req.json()) as Body;
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const auth = getBetterAuth();
  const db = getDb();

  const session = await auth.api.getSession({ headers: req.headers });
  if (!session?.user?.id) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const userId = session.user.id;
  const rate = await consumeRateLimit({
    key: clientKey(req, userId),
    limit: 8,
    windowMs: 15 * 60 * 1000,
  });
  if (!rate.ok) {
    return NextResponse.json(
      { error: `Trop de tentatives. Réessayez dans ${rate.retryAfterSec}s.` },
      { status: 429, headers: { "Retry-After": String(rate.retryAfterSec) } },
    );
  }

  const currentPassword = String(
    "currentPassword" in body ? body.currentPassword ?? "" : "",
  );
  if (currentPassword.length < 8) {
    return NextResponse.json({ error: "Mot de passe actuel requis." }, { status: 400 });
  }

  try {
    await auth.api.verifyPassword({
      body: { password: currentPassword },
      headers: req.headers,
    });
  } catch {
    return NextResponse.json({ error: "Mot de passe actuel incorrect." }, { status: 400 });
  }

  if (body.action === "password") {
    const newPassword = String(body.newPassword ?? "");
    const policy = validatePasswordPolicy(newPassword);
    if (!policy.ok) {
      return NextResponse.json({ error: policy.error }, { status: 400 });
    }
    if (newPassword === currentPassword) {
      return NextResponse.json(
        { error: "Choisissez un mot de passe différent de l’actuel." },
        { status: 400 },
      );
    }
    try {
      await auth.api.changePassword({
        body: {
          currentPassword,
          newPassword,
          revokeOtherSessions: true,
        },
        headers: req.headers,
      });
    } catch {
      const hashed = await hashPassword(newPassword);
      const [cred] = await db
        .select()
        .from(account)
        .where(and(eq(account.userId, userId), eq(account.providerId, "credential")))
        .limit(1);
      if (!cred) {
        return NextResponse.json({ error: "Compte mot de passe introuvable." }, { status: 400 });
      }
      await db
        .update(account)
        .set({ password: hashed, updatedAt: new Date() })
        .where(eq(account.id, cred.id));
    }

    await db
      .update(user)
      .set({ mustChangePassword: false, updatedAt: new Date() })
      .where(eq(user.id, userId));

    await writeSecurityAudit({
      userId,
      action: "password_changed",
      req,
    });

    return NextResponse.json({ ok: true, action: "password" });
  }

  if (body.action === "email") {
    const newEmail = String(body.newEmail ?? "").trim().toLowerCase();
    if (!isEmail(newEmail)) {
      return NextResponse.json({ error: "Adresse e-mail invalide." }, { status: 400 });
    }
    if (newEmail === session.user.email.toLowerCase()) {
      return NextResponse.json({ error: "C’est déjà votre e-mail actuel." }, { status: 400 });
    }

    const [taken] = await db
      .select({ id: user.id })
      .from(user)
      .where(and(sql`lower(${user.email}) = ${newEmail}`, ne(user.id, userId)))
      .limit(1);
    if (taken) {
      return NextResponse.json({ error: "Cet e-mail est déjà utilisé." }, { status: 409 });
    }

    const rawToken = randomBytes(32).toString("hex");
    const tokenHash = createHash("sha256").update(rawToken).digest("hex");
    const expiresAt = new Date(Date.now() + 60 * 60 * 1000);

    await db.delete(verification).where(like(verification.identifier, `email-change:${userId}:%`));
    await db.insert(verification).values({
      id: crypto.randomUUID(),
      identifier: `email-change:${userId}:${newEmail}`,
      value: tokenHash,
      expiresAt,
    });

    const confirmUrl = `${requestOrigin(req)}/auth/confirm-email-change?token=${encodeURIComponent(rawToken)}`;
    const mailed = await sendSecurityMail({
      to: newEmail,
      subject: "Confirmez votre nouvel e-mail ScolIA",
      text: `Bonjour,\n\nPour confirmer le changement d'e-mail de connexion ScolIA, ouvrez ce lien (valable 1 h) :\n\n${confirmUrl}\n\nSi vous n'êtes pas à l'origine de cette demande, ignorez ce message.\n`,
    });

    if (!mailed) {
      // Sans SMTP : bascule immédiate mais sessions révoquées + notification best-effort.
      await db
        .update(user)
        .set({ email: newEmail, emailVerified: true, updatedAt: new Date() })
        .where(eq(user.id, userId));
      await db.delete(verification).where(like(verification.identifier, `email-change:${userId}:%`));
      try {
        await auth.api.revokeOtherSessions({ headers: req.headers });
      } catch {
        /* ignore */
      }
      await writeSecurityAudit({
        userId,
        action: "email_change_immediate",
        req,
        metadata: { newEmail },
      });
      return NextResponse.json({
        ok: true,
        action: "email",
        email: newEmail,
        mode: "immediate",
        warning:
          "E-mail mis à jour immédiatement (envoi de confirmation indisponible). Reconnectez-vous si besoin.",
      });
    }

    void sendSecurityMail({
      to: session.user.email,
      subject: "Demande de changement d’e-mail ScolIA",
      text: `Bonjour,\n\nUne demande de changement d'e-mail vers ${newEmail} a été initiée sur votre compte.\nSi ce n'est pas vous, changez votre mot de passe immédiatement.\n`,
    });

    await writeSecurityAudit({
      userId,
      action: "email_change_requested",
      req,
      metadata: { newEmail },
    });

    return NextResponse.json({
      ok: true,
      action: "email",
      mode: "confirm",
      message: `Un e-mail de confirmation a été envoyé à ${newEmail}.`,
    });
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}
