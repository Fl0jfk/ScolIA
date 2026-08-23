import { NextResponse } from "next/server";
import { hashPassword } from "better-auth/crypto";
import { and, eq, sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import { account, user } from "@/db/schema";
import { isBetterAuthConfigured } from "@/app/lib/auth-config";

type ClaimBody = {
  email?: string;
  password?: string;
  firstName?: string;
  lastName?: string;
};

/**
 * Attache un mot de passe Better-Auth à un compte déjà déjà provisionné
 * (ligne `user` sans compte `credential`).
 */
export async function POST(request: Request) {
  if (!isBetterAuthConfigured() || !isDatabaseConfigured()) {
    return NextResponse.json({ error: "Auth indisponible." }, { status: 503 });
  }

  let body: ClaimBody;
  try {
    body = (await request.json()) as ClaimBody;
  } catch {
    return NextResponse.json({ error: "JSON invalide." }, { status: 400 });
  }

  const email = body.email?.trim().toLowerCase() ?? "";
  const password = body.password ?? "";
  if (!email || !password) {
    return NextResponse.json({ error: "E-mail et mot de passe requis." }, { status: 400 });
  }
  if (password.length < 8) {
    return NextResponse.json({ error: "Mot de passe trop court (min. 8)." }, { status: 400 });
  }

  const db = getDb();
  const [row] = await db
    .select()
    .from(user)
    .where(sql`lower(${user.email}) = ${email}`)
    .limit(1);

  if (!row) {
    return NextResponse.json(
      { error: "Compte inconnu. Utilisez « Créer un compte » pour une nouvelle adresse." },
      { status: 404 },
    );
  }

  const [existing] = await db
    .select({ id: account.id })
    .from(account)
    .where(and(eq(account.userId, row.id), eq(account.providerId, "credential")))
    .limit(1);

  if (existing) {
    return NextResponse.json(
      { error: "Un mot de passe existe déjà. Connectez-vous ou réinitialisez-le." },
      { status: 409 },
    );
  }

  const hashed = await hashPassword(password);
  const firstName = body.firstName?.trim() || row.firstName;
  const lastName = body.lastName?.trim() || row.lastName;
  const name =
    `${firstName ?? ""} ${lastName ?? ""}`.trim() || row.name || row.email;

  await db.insert(account).values({
    id: crypto.randomUUID(),
    issuer: "local:credential",
    accountId: row.id,
    providerId: "credential",
    userId: row.id,
    password: hashed,
  });

  await db
    .update(user)
    .set({
      firstName: firstName ?? null,
      lastName: lastName ?? null,
      name,
      emailVerified: true,
      updatedAt: new Date(),
    })
    .where(eq(user.id, row.id));

  return NextResponse.json({ ok: true, email: row.email });
}
