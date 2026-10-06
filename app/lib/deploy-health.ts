import "server-only";

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { sql } from "drizzle-orm";
import { getDb, isDatabaseConfigured } from "@/db/index";

type JournalEntry = { tag: string; when: number };

function loadJournalEntries(): JournalEntry[] {
  const journalPath = path.join(process.cwd(), "drizzle", "meta", "_journal.json");
  const raw = JSON.parse(fs.readFileSync(journalPath, "utf8")) as {
    entries: JournalEntry[];
  };
  return raw.entries;
}

function buildHashAndTagIndex(entries: JournalEntry[]): Map<string, string> {
  const root = process.cwd();
  const index = new Map<string, string>();
  for (const entry of entries) {
    index.set(entry.tag, entry.tag);
    const file = path.join(root, "drizzle", `${entry.tag}.sql`);
    if (fs.existsSync(file)) {
      const content = fs.readFileSync(file, "utf8");
      const hash = crypto.createHash("sha256").update(content).digest("hex");
      index.set(hash, entry.tag);
    }
  }
  return index;
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
  const rows = await db.execute<{ hash: string }>(sql`
    SELECT hash
    FROM drizzle.__drizzle_migrations
    ORDER BY created_at DESC, id DESC
    LIMIT 1
  `);

  const lastHash = rows[0]?.hash;
  if (!lastHash) {
    return { ok: true, gitSha, migrationTag: null };
  }

  const index = buildHashAndTagIndex(loadJournalEntries());
  const migrationTag = index.get(lastHash) ?? null;
  return { ok: true, gitSha, migrationTag };
}
