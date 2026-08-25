import { readFileSync, existsSync } from "node:fs";
import postgres from "postgres";

function loadEnvFile(path) {
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
process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";

const raw = process.env.DATABASE_URL?.trim();
if (!raw) {
  console.error("DATABASE_URL manquante dans .env.local");
  process.exit(1);
}

const u = new URL(raw.replace(/^postgresql:/, "postgres:"));
console.log(`Connexion ${u.hostname}:${u.port || "5432"}…`);

const sql = postgres(raw, { max: 1, connect_timeout: 30, ssl: "require" });

try {
  const ping = await sql`select current_database() as db`;
  console.log(`Connecté db=${ping[0].db}`);
  await sql.file("drizzle/0013_user_membership.sql");
  const count = await sql`select count(*)::int as n from user_membership`;
  console.log(`OK — user_membership rows=${count[0].n}`);
} catch (e) {
  console.error("Échec:", e instanceof Error ? e.message : e);
  process.exitCode = 1;
} finally {
  await sql.end({ timeout: 5 });
}
