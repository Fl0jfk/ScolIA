/**
 * Définit (ou remplace) le mot de passe Better-Auth d’un utilisateur déjà en BDD
 * (ex. déjà provisionné, sans compte `credential`).
 *
 * Usage :
 *   npm run auth:set-password -- --email=toi@exemple.fr --password='MotDePasseSecure'
 */
import { existsSync, readFileSync } from "node:fs";
import { hashPassword } from "better-auth/crypto";
import { and, eq, sql } from "drizzle-orm";
import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import * as schema from "../db/schema";
import { account, user } from "../db/schema";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx <= 0) continue;
    const key = trimmed.slice(0, eqIdx);
    let value = trimmed.slice(eqIdx + 1);
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = value;
  }
}

loadEnvFile(".env.local");
loadEnvFile(".env");

function argValue(name: string): string | undefined {
  const prefix = `--${name}=`;
  const hit = process.argv.find((a) => a.startsWith(prefix));
  return hit?.slice(prefix.length);
}

async function main() {
  const email = argValue("email")?.trim().toLowerCase();
  const password = argValue("password");
  if (!email || !password) {
    console.error("Usage : --email=<email> --password=<motdepasse>");
    process.exit(1);
  }
  if (password.length < 8) {
    console.error("Mot de passe trop court (min. 8 caractères).");
    process.exit(1);
  }

  const databaseUrl = process.env.DATABASE_URL?.trim();
  if (!databaseUrl) {
    console.error("DATABASE_URL manquante");
    process.exit(1);
  }

  const client = postgres(databaseUrl, { max: 2, prepare: false });
  const db = drizzle(client, { schema });

  try {
    const [row] = await db
      .select()
      .from(user)
      .where(sql`lower(${user.email}) = ${email}`)
      .limit(1);

    if (!row) {
      console.error(`Aucun utilisateur pour ${email}`);
      process.exit(1);
    }

    const hashed = await hashPassword(password);
    const [existing] = await db
      .select()
      .from(account)
      .where(and(eq(account.userId, row.id), eq(account.providerId, "credential")))
      .limit(1);

    if (existing) {
      await db
        .update(account)
        .set({
          password: hashed,
          issuer: "local:credential",
          accountId: row.id,
          updatedAt: new Date(),
        })
        .where(eq(account.id, existing.id));
      console.log(JSON.stringify({ email: row.email, userId: row.id, action: "updated" }, null, 2));
    } else {
      await db.insert(account).values({
        id: crypto.randomUUID(),
        issuer: "local:credential",
        accountId: row.id,
        providerId: "credential",
        userId: row.id,
        password: hashed,
      });
      console.log(JSON.stringify({ email: row.email, userId: row.id, action: "created" }, null, 2));
    }
  } finally {
    await client.end({ timeout: 5 });
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
