/**
 * Applique les migrations SQL en direct.
 * Gère l'historique mixte (hash SHA vs tag) : backfill les tags déjà en prod sans re-jouer le SQL.
 */
import fs from "node:fs";
import path from "node:path";
import postgres from "postgres";

const root = path.resolve(import.meta.dirname, "..");
const journalPath = path.join(root, "drizzle", "meta", "_journal.json");
const journal = JSON.parse(fs.readFileSync(journalPath, "utf8"));

/** Dernière migration déjà appliquée en prod (objets existants, journal incohérent). */
const BACKFILL_UNTIL_TAG = "0013_user_membership";

const url = process.env.DATABASE_URL;
if (!url) {
  console.error("DATABASE_URL manquant.");
  process.exit(1);
}

const sql = postgres(url, {
  ssl: process.env.NODE_TLS_REJECT_UNAUTHORIZED === "0" ? { rejectUnauthorized: false } : undefined,
  connect_timeout: 120,
  idle_timeout: 120,
  max: 1,
});

async function ensureMigrationsTable() {
  await sql`
    CREATE TABLE IF NOT EXISTS drizzle.__drizzle_migrations (
      id SERIAL PRIMARY KEY,
      hash text NOT NULL,
      created_at bigint
    )
  `;
}

async function appliedHashes() {
  const rows = await sql`SELECT hash FROM drizzle.__drizzle_migrations ORDER BY created_at`;
  return new Set(rows.map((r) => r.hash));
}

function splitStatements(fileContent) {
  return fileContent
    .split(/--> statement-breakpoint\n?/g)
    .map((s) => s.trim())
    .filter(Boolean);
}

function isApplied(tag, done) {
  return done.has(tag);
}

async function backfillTag(tag, done) {
  if (isApplied(tag, done)) return;
  await sql`INSERT INTO drizzle.__drizzle_migrations (hash, created_at) VALUES (${tag}, ${Date.now()})`;
  done.add(tag);
  console.log(`backfill ${tag}`);
}

async function applyFile(tag, done) {
  if (isApplied(tag, done)) {
    console.log(`skip ${tag}`);
    return;
  }
  const file = path.join(root, "drizzle", `${tag}.sql`);
  if (!fs.existsSync(file)) {
    console.warn(`missing ${file}`);
    return;
  }
  console.log(`apply ${tag}…`);
  const content = fs.readFileSync(file, "utf8");
  for (const stmt of splitStatements(content)) {
    await sql.unsafe(stmt);
  }
  await sql`INSERT INTO drizzle.__drizzle_migrations (hash, created_at) VALUES (${tag}, ${Date.now()})`;
  done.add(tag);
  console.log(`ok ${tag}`);
}

async function main() {
  await sql`CREATE SCHEMA IF NOT EXISTS drizzle`;
  await ensureMigrationsTable();
  const done = await appliedHashes();

  let backfillMode = true;
  for (const entry of journal.entries) {
    const tag = entry.tag;
    if (backfillMode) {
      await backfillTag(tag, done);
      if (tag === BACKFILL_UNTIL_TAG) backfillMode = false;
      continue;
    }
    await applyFile(tag, done);
  }

  console.log("Migrations terminées.");
  await sql.end({ timeout: 5 });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
