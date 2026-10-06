import "server-only";

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";
import {
  type JournalEntry,
  resolveLatestMigrationTag,
} from "@/app/lib/deploy-health-logic";

let cachedJournalEntries: JournalEntry[] | null = null;
let cachedTagToContentHash: Map<string, string> | null = null;
let cachedMigrationHashIndex: Map<string, string> | null = null;

function loadJournalEntries(): JournalEntry[] {
  if (cachedJournalEntries) return cachedJournalEntries;
  const journalPath = path.join(process.cwd(), "drizzle", "meta", "_journal.json");
  const raw = JSON.parse(fs.readFileSync(journalPath, "utf8")) as {
    entries: JournalEntry[];
  };
  cachedJournalEntries = raw.entries;
  return cachedJournalEntries;
}

function buildTagToContentHash(entries: JournalEntry[]): Map<string, string> {
  const root = process.cwd();
  const index = new Map<string, string>();
  for (const entry of entries) {
    const file = path.join(root, "drizzle", `${entry.tag}.sql`);
    if (fs.existsSync(file)) {
      const content = fs.readFileSync(file, "utf8");
      const hash = crypto.createHash("sha256").update(content).digest("hex");
      index.set(entry.tag, hash);
    }
  }
  return index;
}

function getTagToContentHash(): Map<string, string> {
  if (!cachedTagToContentHash) {
    cachedTagToContentHash = buildTagToContentHash(loadJournalEntries());
  }
  return cachedTagToContentHash;
}

function buildHashAndTagIndex(entries: JournalEntry[]): Map<string, string> {
  const tagToHash = buildTagToContentHash(entries);
  const index = new Map<string, string>();
  for (const entry of entries) {
    index.set(entry.tag, entry.tag);
    const hash = tagToHash.get(entry.tag);
    if (hash) index.set(hash, entry.tag);
  }
  return index;
}

function getMigrationHashIndex(): Map<string, string> {
  if (!cachedMigrationHashIndex) {
    cachedMigrationHashIndex = buildHashAndTagIndex(loadJournalEntries());
  }
  return cachedMigrationHashIndex;
}

export type DeployHealthPayload = {
  ok: true;
  gitSha: string;
  migrationTag: string | null;
};

export async function getDeployHealthPayload(): Promise<DeployHealthPayload> {
  const gitSha = process.env.GIT_SHA?.trim() || "unknown";
  if (!isDatabaseConfigured()) {
    return { ok: true, gitSha, migrationTag: null };
  }

  const db = getDb();
  const appliedRows = await db.execute<{ hash: string }>(sql`
    SELECT hash
    FROM drizzle.__drizzle_migrations
  `);
  const appliedHashes = appliedRows.map((row) => row.hash);
  if (appliedHashes.length === 0) {
    return { ok: true, gitSha, migrationTag: null };
  }

  const journalEntries = loadJournalEntries();
  let migrationTag = resolveLatestMigrationTag(
    journalEntries,
    getTagToContentHash(),
    appliedHashes,
  );

  if (!migrationTag) {
    const fallbackRows = await db.execute<{ hash: string }>(sql`
      SELECT hash
      FROM drizzle.__drizzle_migrations
      ORDER BY id DESC
      LIMIT 1
    `);
    const fallbackHash = fallbackRows[0]?.hash;
    migrationTag = fallbackHash ? getMigrationHashIndex().get(fallbackHash) ?? null : null;
  }

  return { ok: true, gitSha, migrationTag };
}
