/**
 * Ajoute last_delivered_at / last_delivered_message_id sur messaging_participant.
 *   node scripts/ensure-messaging-delivery.mjs
 */
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
loadEnvFile(".env");

const url = process.env.DATABASE_URL;
if (!url) throw new Error("DATABASE_URL manquant");

const sql = postgres(url, {
  max: 1,
  ssl: /scaleway|amazonaws/i.test(url) ? "require" : false,
});

await sql.unsafe(`
  ALTER TABLE messaging_participant
    ADD COLUMN IF NOT EXISTS last_delivered_at timestamptz;
  ALTER TABLE messaging_participant
    ADD COLUMN IF NOT EXISTS last_delivered_message_id text;
`);

const cols = await sql`
  SELECT column_name, data_type
  FROM information_schema.columns
  WHERE table_name = 'messaging_participant'
    AND column_name IN ('last_delivered_at', 'last_delivered_message_id')
  ORDER BY column_name
`;
console.log(JSON.stringify({ ok: true, columns: cols }, null, 2));
await sql.end();
