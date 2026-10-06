/**
 * Crée les tables messaging_* si absentes.
 *   node --import tsx scripts/ensure-messaging-tables.ts
 */
import { readFileSync, existsSync } from "node:fs";
import path from "node:path";
import postgres from "postgres";

const messagingCoreDdl = readFileSync(
  path.join(import.meta.dirname, "messaging-core-ddl.sql"),
  "utf8",
);

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq <= 0) continue;
    const key = trimmed.slice(0, eq);
    let value = trimmed.slice(eq + 1);
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

const url = process.env.DATABASE_URL;
if (!url) throw new Error("DATABASE_URL manquant");

const needsSsl =
  /scaleway|amazonaws|neon|supabase|render\.com/i.test(url) ||
  process.env.PGSSLMODE === "require";

const sql = postgres(url, {
  max: 1,
  ssl: needsSsl ? "require" : false,
});

async function main() {
  await sql.unsafe(messagingCoreDdl);

  const tables = await sql`
    SELECT table_name FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name LIKE 'messaging_%'
    ORDER BY table_name
  `;
  console.log(JSON.stringify({ ok: true, tables: tables.map((t) => t.table_name) }, null, 2));
  await sql.end();
}

main().catch(async (err) => {
  console.error(err);
  try {
    await sql.end({ timeout: 1 });
  } catch {
    /* ignore */
  }
  process.exit(1);
});
