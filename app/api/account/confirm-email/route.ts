import { NextResponse } from "next/server";
import { createHash } from "crypto";
import { and, eq, ne, sql } from "drizzle-orm";
import { createPlatformTransporter } from "@/app/lib/tenant-mail";
import { getDb } from "@/db/index";
import { user, verification } from "@/db/schema";

function isEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

async function notify(to: string, subject: string, text: string) {
  const transporter = createPlatformTransporter();
  if (!transporter) return;
  try {
    const from =
      process.env.MAILER_EMAIL?.trim() ||
      process.env.SMTP_USER?.trim() ||
      "mailer@scolia.fr";
    await transporter.sendMail({ from: `"ScolIA" <${from}>`, to, subject, text });
  } catch (error) {
    console.error("[confirm-email] mail", error);
  }
}

/** Confirmation publique du changement d’e-mail (jeton one-shot). */
export async function POST(req: Request) {
  let token = "";
  try {
    const body = (await req.json()) as { token?: string };
    token = String(body.token ?? "").trim();
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }
  if (!token) {
    return NextResponse.json({ error: "Jeton manquant." }, { status: 400 });
  }

  const db = getDb();
  const tokenHash = createHash("sha256").update(token).digest("hex");
  const [row] = await db
    .select()
    .from(verification)
    .where(eq(verification.value, tokenHash))
    .limit(1);
  if (!row || row.expiresAt.getTime() < Date.now()) {
    return NextResponse.json({ error: "Lien expiré ou invalide." }, { status: 400 });
  }
  if (!row.identifier.startsWith("email-change:")) {
    return NextResponse.json({ error: "Jeton invalide." }, { status: 400 });
  }
  const parts = row.identifier.split(":");
  const uid = parts[1] ?? "";
  const email = parts.slice(2).join(":").toLowerCase();
  if (!uid || !isEmail(email)) {
    return NextResponse.json({ error: "Jeton corrompu." }, { status: 400 });
  }

  const [taken] = await db
    .select({ id: user.id })
    .from(user)
    .where(and(sql`lower(${user.email}) = ${email}`, ne(user.id, uid)))
    .limit(1);
  if (taken) {
    return NextResponse.json({ error: "Cet e-mail est déjà utilisé." }, { status: 409 });
  }

  const [before] = await db.select().from(user).where(eq(user.id, uid)).limit(1);
  await db
    .update(user)
    .set({ email, emailVerified: true, updatedAt: new Date() })
    .where(eq(user.id, uid));
  await db.delete(verification).where(eq(verification.id, row.id));

  if (before?.email) {
    void notify(
      before.email,
      "Votre e-mail ScolIA a été modifié",
      `Bonjour,\n\nL'adresse de connexion de votre compte ScolIA a été changée pour ${email}.\nSi vous n'êtes pas à l'origine de cette action, contactez immédiatement votre établissement.\n`,
    );
  }

  return NextResponse.json({ ok: true, email });
}
