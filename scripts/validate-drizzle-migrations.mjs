/**
 * Garde-fou CI : cohérence drizzle/meta/_journal.json ↔ fichiers SQL.
 * Échoue si une migration serait ignorée par drizzle-kit migrate (when en doublon / non monotone).
 */
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const drizzleDir = path.join(root, "drizzle");
const journalPath = path.join(drizzleDir, "meta", "_journal.json");

function fail(message) {
  console.error(`::error::${message}`);
  process.exitCode = 1;
}

function warn(message) {
  console.warn(`::warning::${message}`);
}

if (!fs.existsSync(journalPath)) {
  fail(`Journal introuvable : ${journalPath}`);
  process.exit(1);
}

const journal = JSON.parse(fs.readFileSync(journalPath, "utf8"));
const entries = journal.entries ?? [];

const sqlFiles = fs
  .readdirSync(drizzleDir)
  .filter((name) => name.endsWith(".sql"))
  .map((name) => name.replace(/\.sql$/, ""))
  .sort();

const tags = entries.map((e) => e.tag);
const tagSet = new Set(tags);

for (const tag of sqlFiles) {
  if (!tagSet.has(tag)) {
    fail(`Fichier SQL absent du journal : ${tag}.sql`);
  }
}

for (const entry of entries) {
  const file = path.join(drizzleDir, `${entry.tag}.sql`);
  if (!fs.existsSync(file)) {
    fail(`Entrée journal sans fichier SQL : ${entry.tag}`);
  }
}

for (let i = 0; i < entries.length; i++) {
  if (entries[i].idx !== i) {
    fail(`idx journal incohérent : attendu ${i}, trouvé ${entries[i].idx} (${entries[i].tag})`);
  }
}

const duplicateTags = tags.filter((t, i) => tags.indexOf(t) !== i);
if (duplicateTags.length > 0) {
  fail(`Tags journal en doublon : ${[...new Set(duplicateTags)].join(", ")}`);
}

const whenByValue = new Map();
for (const entry of entries) {
  const list = whenByValue.get(entry.when) ?? [];
  list.push(entry.tag);
  whenByValue.set(entry.when, list);
}
for (const [when, list] of whenByValue) {
  if (list.length > 1) {
    fail(
      `Champ journal "when" en doublon (${when}) — drizzle-kit migrate ignore les migrations suivantes : ${list.join(", ")}`,
    );
  }
}

for (let i = 1; i < entries.length; i++) {
  if (entries[i].when <= entries[i - 1].when) {
    fail(
      `"when" non strictement croissant entre ${entries[i - 1].tag} (${entries[i - 1].when}) et ${entries[i].tag} (${entries[i].when})`,
    );
  }
}

const prefixCounts = new Map();
for (const tag of tags) {
  const m = /^(\d{4})_/.exec(tag);
  if (!m) continue;
  const prefix = m[1];
  prefixCounts.set(prefix, (prefixCounts.get(prefix) ?? 0) + 1);
}
const dupPrefixes = [...prefixCounts.entries()].filter(([, n]) => n > 1).map(([p]) => p);
if (dupPrefixes.length > 0) {
  warn(
    `Préfixes numériques partagés (OK si tags uniques, mais risque de confusion) : ${dupPrefixes.join(", ")}`,
  );
}

if (process.exitCode === 1) {
  console.error("Validation migrations Drizzle : ÉCHEC");
  process.exit(1);
}

console.log(
  `Validation migrations Drizzle : OK (${entries.length} entrées journal, ${sqlFiles.length} fichiers SQL)`,
);
