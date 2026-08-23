import { NextResponse } from "next/server";
import { and, eq, ne, sql } from "drizzle-orm";
import { hashPassword } from "better-auth/crypto";
import { getBetterAuth } from "@/app/lib/auth-server";
import { getDb } from "@/db/index";
import { account, user } from "@/db/schema";

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

/** Mise à jour e-mail / mot de passe du compte connecté. */
export async function POST(req: Request) {
  let body: Body;
  try {
    body = (await req.json()) as Body;
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const auth = getBetterAuth();
  const session = await auth.api.getSession({ headers: req.headers });
  if (!session?.user?.id) {
    return NextResponse.json({ error: "Non authentifié." }, { status: 401 });
  }

  const userId = session.user.id;
  const currentPassword = String(body.currentPassword ?? "");
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

  const db = getDb();

  if (body.action === "password") {
    const newPassword = String(body.newPassword ?? "");
    if (newPassword.length < 8) {
      return NextResponse.json(
        { error: "Le nouveau mot de passe doit contenir au moins 8 caractères." },
        { status: 400 },
      );
    }
    try {
      await auth.api.changePassword({
        body: {
          currentPassword,
          newPassword,
          revokeOtherSessions: false,
        },
        headers: req.headers,
      });
    } catch {
      // Repli direct si l’API changePassword échoue (compte credential atypique).
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

    await db
      .update(user)
      .set({
        email: newEmail,
        emailVerified: true,
        updatedAt: new Date(),
      })
      .where(eq(user.id, userId));

    return NextResponse.json({ ok: true, action: "email", email: newEmail });
  }

  return NextResponse.json({ error: "Action inconnue." }, { status: 400 });
}
