/**
 * Tables signalisation messagerie (NOTIFY gros payloads + sonneries d’appel).
 *   node --import tsx scripts/ensure-messaging-call-relay.mjs
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

// purge vieux relais
await sql`DELETE FROM messaging_signal_relay WHERE created_at < now() - interval '10 minutes'`;
await sql`UPDATE messaging_call_ring SET status = 'expired' WHERE status = 'ringing' AND expires_at < now()`;

console.log(JSON.stringify({ ok: true }, null, 2));
await sql.end();
