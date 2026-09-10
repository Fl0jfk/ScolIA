/**
 * Migration messagerie V2 : groupes (kind/title/created_by) + user_a/b nullable.
 *   node --import tsx scripts/ensure-messaging-v2.mjs
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
  ALTER TABLE messaging_conversation
    ADD COLUMN IF NOT EXISTS kind text NOT NULL DEFAULT 'dm';
  ALTER TABLE messaging_conversation
    ADD COLUMN IF NOT EXISTS title text;
  ALTER TABLE messaging_conversation
    ADD COLUMN IF NOT EXISTS created_by_id text;

  ALTER TABLE messaging_conversation
    ALTER COLUMN user_a_id DROP NOT NULL;
  ALTER TABLE messaging_conversation
    ALTER COLUMN user_b_id DROP NOT NULL;

  DROP INDEX IF EXISTS messaging_conversation_pair_uidx;
  CREATE UNIQUE INDEX IF NOT EXISTS messaging_conversation_dm_pair_uidx
    ON messaging_conversation (etablissement_id, user_a_id, user_b_id)
    WHERE kind = 'dm';
  CREATE INDEX IF NOT EXISTS messaging_conversation_kind_idx
    ON messaging_conversation (etablissement_id, kind);

  UPDATE messaging_conversation SET kind = 'dm' WHERE kind IS NULL OR kind = '';
`);

const cols = await sql`
  SELECT column_name, is_nullable, data_type
  FROM information_schema.columns
  WHERE table_name = 'messaging_conversation'
    AND column_name IN ('kind','title','created_by_id','user_a_id','user_b_id')
  ORDER BY column_name
`;
console.log(JSON.stringify({ ok: true, columns: cols }, null, 2));
await sql.end();
