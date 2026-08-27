/**
 * Applique le correctif MFA (verified DEFAULT false + purge setups incomplets).
 */
import { existsSync, readFileSync } from "node:fs";
import { sql } from "drizzle-orm";
import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";

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

async function main() {
  const url = process.env.DATABASE_URL?.trim();
  if (!url) {
    console.error("DATABASE_URL manquante");
    process.exit(1);
  }
  const client = postgres(url, { max: 1, prepare: false });
  const db = drizzle(client);
  try {
    await db.execute(
      sql.raw(`ALTER TABLE "two_factor" ALTER COLUMN "verified" SET DEFAULT false`),
    );
    await db.execute(
      sql.raw(`
        DELETE FROM "two_factor" AS tf
        USING "user" AS u
        WHERE tf."user_id" = u."id"
          AND u."two_factor_enabled" = false
      `),
    );
    await db.execute(
      sql.raw(`
        UPDATE "two_factor" AS tf
        SET "verified" = true
        FROM "user" AS u
        WHERE tf."user_id" = u."id"
          AND u."two_factor_enabled" = true
          AND tf."verified" IS DISTINCT FROM true
      `),
    );
    console.log("OK: MFA verified DEFAULT false + purge setups incomplets");
  } finally {
    await client.end({ timeout: 5 });
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
