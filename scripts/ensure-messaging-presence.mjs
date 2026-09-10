/**
 * Table présence messagerie + relais NOTIFY (gros payloads / sonneries).
 *   node --import tsx scripts/ensure-messaging-presence.mjs
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
  CREATE TABLE IF NOT EXISTS messaging_presence (
    user_id text PRIMARY KEY,
    etablissement_id uuid NOT NULL REFERENCES etablissement(id) ON DELETE CASCADE,
    status text NOT NULL DEFAULT 'offline',
    connections integer NOT NULL DEFAULT 0,
    manual_status text,
    manual_until timestamptz,
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
  );
  ALTER TABLE messaging_presence ADD COLUMN IF NOT EXISTS manual_status text;
  ALTER TABLE messaging_presence ADD COLUMN IF NOT EXISTS manual_until timestamptz;
  CREATE INDEX IF NOT EXISTS messaging_presence_etab_idx
    ON messaging_presence (etablissement_id);
  CREATE INDEX IF NOT EXISTS messaging_presence_etab_status_idx
    ON messaging_presence (etablissement_id, status);

  CREATE TABLE IF NOT EXISTS messaging_signal_relay (
    id text PRIMARY KEY,
    event jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
  );
  CREATE INDEX IF NOT EXISTS messaging_signal_relay_created_idx
    ON messaging_signal_relay (created_at);

  CREATE TABLE IF NOT EXISTS messaging_call_ring (
    id text PRIMARY KEY,
    call_id text NOT NULL,
    etablissement_id uuid NOT NULL,
    conversation_id text NOT NULL,
    from_user_id text NOT NULL,
    from_name text NOT NULL,
    to_user_id text NOT NULL,
    title text NOT NULL,
    kind text NOT NULL DEFAULT 'dm',
    status text NOT NULL DEFAULT 'ringing',
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
  );
  CREATE INDEX IF NOT EXISTS messaging_call_ring_to_idx
    ON messaging_call_ring (to_user_id, status, expires_at);
  CREATE INDEX IF NOT EXISTS messaging_call_ring_call_idx
    ON messaging_call_ring (call_id);
`);

await sql`DELETE FROM messaging_signal_relay WHERE created_at < now() - interval '10 minutes'`;
await sql`UPDATE messaging_call_ring SET status = 'expired' WHERE status = 'ringing' AND expires_at < now()`;

const cols = await sql`
  SELECT column_name, data_type
  FROM information_schema.columns
  WHERE table_name = 'messaging_presence'
  ORDER BY ordinal_position
`;
console.log(JSON.stringify({ ok: true, columns: cols }, null, 2));
await sql.end();
