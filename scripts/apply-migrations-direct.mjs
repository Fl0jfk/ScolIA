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

/** Verrou PostgreSQL session — une seule instance applique les migrations à la fois. */
const MIGRATION_ADVISORY_LOCK_KEY1 = 0x5343; // "SC"
const MIGRATION_ADVISORY_LOCK_KEY2 = 0x4f4c; // "OL"

const root = path.resolve(import.meta.dirname, "..");
const journalPath = path.join(root, "drizzle", "meta", "_journal.json");
const messagingCoreDdlPath = path.join(root, "scripts", "messaging-core-ddl.sql");
const messagingCoreDdl = fs.readFileSync(messagingCoreDdlPath, "utf8");
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

/**
 * 0000_initial.sql est déjà post-renommage ; 0002 échoue sur base vierge si on rejoue le SQL tel quel.
 * N’altère pas le fichier (hash prod inchangé) — exécution idempotente à la place.
 */
async function apply0002RenameClerkIdsIdempotent(tx) {
  const userClerk = await tx`
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public' AND table_name = 'user' AND column_name = 'clerk_user_id'
    LIMIT 1
  `;
  if (userClerk.length > 0) {
    await tx.unsafe(
      `ALTER TABLE "user" RENAME COLUMN "clerk_user_id" TO "external_user_id"`,
    );
  }

  await tx.unsafe(
    `ALTER INDEX IF EXISTS "user_clerk_user_id_idx" RENAME TO "user_external_user_id_idx"`,
  );

  const clerkMapping = await tx`
    SELECT 1 FROM information_schema.tables
    WHERE table_schema = 'public' AND table_name = 'clerk_user_mapping'
    LIMIT 1
  `;
  if (clerkMapping.length > 0) {
    await tx.unsafe(`ALTER TABLE "clerk_user_mapping" RENAME TO "auth_user_mapping"`);
  }

  const mappingClerkCol = await tx`
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public' AND table_name = 'auth_user_mapping' AND column_name = 'clerk_user_id'
    LIMIT 1
  `;
  if (mappingClerkCol.length > 0) {
    await tx.unsafe(
      `ALTER TABLE "auth_user_mapping" RENAME COLUMN "clerk_user_id" TO "external_user_id"`,
    );
  }

  await tx.unsafe(
    `ALTER INDEX IF EXISTS "clerk_user_mapping_clerk_uidx" RENAME TO "auth_user_mapping_external_uidx"`,
  );
  await tx.unsafe(
    `ALTER INDEX IF EXISTS "clerk_user_mapping_user_uidx" RENAME TO "auth_user_mapping_user_uidx"`,
  );
}

/**
 * Tables messaging_* : jamais créées par un .sql antérieur à 0056_messaging_delivery (script ensure historique).
 * Sur base vierge : bootstrap IF NOT EXISTS puis ALTER du fichier 0056. En prod déjà journalisée : skip hash/tag.
 */
async function ensureMessagingCoreTables(tx) {
  await tx.unsafe(messagingCoreDdl);
}

async function apply0056MessagingDeliveryIdempotent(tx, statements) {
  await ensureMessagingCoreTables(tx);
  for (const stmt of statements) {
    await tx.unsafe(stmt);
  }
}

async function runMigrationStatements(tx, tag, statements) {
  if (tag === "0002_rename_clerk_ids") {
    await apply0002RenameClerkIdsIdempotent(tx);
    return;
  }
  if (tag === "0056_messaging_delivery") {
    await apply0056MessagingDeliveryIdempotent(tx, statements);
    return;
  }
  for (const stmt of statements) {
    await tx.unsafe(stmt);
  }
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
    await runMigrationStatements(tx, tag, statements);
    await tx`
      INSERT INTO drizzle.__drizzle_migrations (hash, created_at)
      VALUES (${hash}, ${entry.when})
    `;
  });

  done.add(hash);
  done.add(tag);
  console.log(`ok ${tag}`);
}

async function runMigrations() {
  await sql`CREATE SCHEMA IF NOT EXISTS drizzle`;
  await ensureMigrationsTable();
  const done = await appliedHashes();

  for (const entry of journal.entries) {
    await applyEntry(entry, done);
  }

  console.log("Migrations terminées.");
}

async function main() {
  console.log("[migrations] Acquisition du verrou advisory (timeout 5 min)…");
  await sql`SET lock_timeout = '300s'`;
  try {
    await sql`SELECT pg_advisory_lock(${MIGRATION_ADVISORY_LOCK_KEY1}, ${MIGRATION_ADVISORY_LOCK_KEY2})`;
  } catch (lockErr) {
    console.error(
      "[migrations] Verrou advisory indisponible après 5 min — une autre instance applique peut-être les migrations.",
    );
    throw lockErr;
  }
  try {
    await runMigrations();
  } finally {
    try {
      await sql`SELECT pg_advisory_unlock(${MIGRATION_ADVISORY_LOCK_KEY1}, ${MIGRATION_ADVISORY_LOCK_KEY2})`;
    } catch (unlockErr) {
      console.warn("[migrations] pg_advisory_unlock:", unlockErr);
    }
    await sql.end({ timeout: 5 });
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
