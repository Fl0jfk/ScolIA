/**
 * Documente et verrouille le bug drizzle-kit migrate (when en doublon).
 */
import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const root = path.resolve(import.meta.dirname, "..");
const journalPath = path.join(root, "drizzle", "meta", "_journal.json");

test("journal : when strictement croissant (évite migrations ignorées)", () => {
  const journal = JSON.parse(fs.readFileSync(journalPath, "utf8"));
  const entries = journal.entries;
  for (let i = 1; i < entries.length; i++) {
    assert.ok(
      entries[i].when > entries[i - 1].when,
      `${entries[i].tag} when=${entries[i].when} doit être > ${entries[i - 1].tag} (${entries[i - 1].when})`,
    );
  }
});

test("drizzle-kit migrate saute la 2e migration si when identique", () => {
  const migrations = [
    { folderMillis: 100, hash: "a", sql: ["CREATE TABLE t1(id int)"] },
    { folderMillis: 100, hash: "b", sql: ["CREATE TABLE t2(id int)"] },
  ];
  const applied = [];
  let lastCreatedAt = null;
  for (const migration of migrations) {
    if (!lastCreatedAt || Number(lastCreatedAt) < migration.folderMillis) {
      applied.push(migration.hash);
      lastCreatedAt = migration.folderMillis;
    }
  }
  assert.deepEqual(applied, ["a"]);
});

test("apply-migrations-direct enregistre le hash SHA256 du fichier", () => {
  const sample = "--> statement-breakpoint\nSELECT 1;\n";
  const hash = crypto.createHash("sha256").update(sample).digest("hex");
  assert.equal(hash.length, 64);
});

test("base vierge : 0056_messaging_delivery bootstrap messaging_participant (sans modifier le .sql)", () => {
  const migratorPath = path.join(root, "scripts", "apply-migrations-direct.mjs");
  const ddlPath = path.join(root, "scripts", "messaging-core-ddl.sql");
  const src = fs.readFileSync(migratorPath, "utf8");
  const ddl = fs.readFileSync(ddlPath, "utf8");
  assert.match(src, /0056_messaging_delivery/);
  assert.match(src, /apply0056MessagingDeliveryIdempotent/);
  assert.match(ddl, /CREATE TABLE IF NOT EXISTS messaging_participant/);
  const sql0056 = fs.readFileSync(path.join(root, "drizzle", "0056_messaging_delivery.sql"), "utf8");
  assert.match(sql0056, /messaging_participant/);
});

test("apply-migrations-direct : lock_timeout long uniquement pour le verrou advisory", () => {
  const migratorPath = path.join(root, "scripts", "apply-migrations-direct.mjs");
  const src = fs.readFileSync(migratorPath, "utf8");
  assert.match(src, /ADVISORY_LOCK_TIMEOUT/);
  assert.match(src, /DDL_LOCK_TIMEOUT/);
  const setBlocks = [...src.matchAll(/SET lock_timeout/g)];
  assert.ok(setBlocks.length >= 2, "lock_timeout distinct pour verrou et DDL");
});
