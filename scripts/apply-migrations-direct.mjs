/**
 * Applique les migrations SQL listées dans drizzle/meta/_journal.json.
 *
 * Ne repose pas sur drizzle-kit migrate : celui-ci saute toute migration dont le champ
 * journal `when` est ≤ au created_at de la dernière ligne __drizzle_migrations (doublons de `when` = migrations jamais jouées).
 *
 * Journal : enregistre le hash SHA256 du fichier SQL (compatible drizzle-kit). Les anciennes
 * lignes tag-only (scripts legacy) sont reconnues comme déjà appliquées.
 *
 * Prod / conteneur : SCOLA_AUTO_MIGRATE=1 (entrypoint Docker) ou ALLOW_PROD_MIGRATION=1 (manuel).
 */
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { assertLocalDatabase } from "./assert-local-database.mjs";

const root = path.resolve(import.meta.dirname, "..");
const journalPath = path.join(root, "drizzle", "meta", "_journal.json");
const journal = JSON.parse(fs.readFileSync(journalPath, "utf8"));

const url = process.env.DATABASE_URL;
if (!url) {
  console.error("DATABASE_URL manquant.");
  process.exit(1);
}
assertLocalDatabase(url, { action: "apply-migrations-direct" });

const postgres = (await import("postgres")).default;
const sql = postgres(url, {
  ssl: process.env.NODE_TLS_REJECT_UNAUTHORIZED === "0" ? { rejectUnauthorized: false } : undefined,
  connect_timeout: 120,
  idle_timeout: 120,
  max: 1,
});

function contentHash(fileContent) {
  return crypto.createHash("sha256").update(fileContent).digest("hex");
}

function splitStatements(fileContent) {
  return fileContent
    .split(/--> statement-breakpoint\n?/g)
    .map((s) => s.trim())
    .filter(Boolean);
}

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
  const rows = await sql`SELECT hash FROM drizzle.__drizzle_migrations`;
  return new Set(rows.map((r) => r.hash));
}

function isRecorded(tag, hash, done) {
  return done.has(hash) || done.has(tag);
}

async function applyEntry(entry, done) {
  const tag = entry.tag;
  const file = path.join(root, "drizzle", `${tag}.sql`);
  if (!fs.existsSync(file)) {
    throw new Error(`Fichier migration manquant : ${file}`);
  }
  const content = fs.readFileSync(file, "utf8");
  const hash = contentHash(content);

  if (isRecorded(tag, hash, done)) {
    console.log(`skip ${tag}`);
    return;
  }

  console.log(`apply ${tag}…`);
  const statements = splitStatements(content);
  if (statements.length === 0) {
    throw new Error(`Migration vide : ${tag}`);
  }

  await sql.begin(async (tx) => {
    for (const stmt of statements) {
      await tx.unsafe(stmt);
    }
    await tx`
      INSERT INTO drizzle.__drizzle_migrations (hash, created_at)
      VALUES (${hash}, ${entry.when})
    `;
  });

  done.add(hash);
  done.add(tag);
  console.log(`ok ${tag}`);
}

async function main() {
  await sql`CREATE SCHEMA IF NOT EXISTS drizzle`;
  await ensureMigrationsTable();
  const done = await appliedHashes();

  for (const entry of journal.entries) {
    await applyEntry(entry, done);
  }

  console.log("Migrations terminées.");
  await sql.end({ timeout: 5 });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
