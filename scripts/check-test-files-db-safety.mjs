/**
 * Échoue si un fichier *.test.* lit DATABASE_URL (interdit — utiliser TEST_DATABASE_URL + garde).
 */
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const forbidden = /process\.env\.DATABASE_URL|process\.env\[['"]DATABASE_URL['"]\]/;
const allowFiles = new Set([
  "app/lib/test-database-harness.ts",
  "scripts/test-database-guard.mjs",
]);

function lineReadsDatabaseUrl(line) {
  const trimmed = line.trim();
  if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("*")) return false;
  if (/delete\s+process\.env\.DATABASE_URL/.test(line)) return false;
  if (/process\.env\.DATABASE_URL\s*=/.test(line)) return false;
  return forbidden.test(line);
}

function walk(dir, out = []) {
  for (const name of fs.readdirSync(dir, { withFileTypes: true })) {
    if (name.name === "node_modules" || name.name === ".next") continue;
    const full = path.join(dir, name.name);
    if (name.isDirectory()) walk(full, out);
    else if (/\.test\.(ts|tsx|mjs|js)$/.test(name.name)) out.push(full);
  }
  return out;
}

const violations = [];
for (const file of walk(root)) {
  const rel = path.relative(root, file);
  if (rel === "scripts/check-test-files-db-safety.mjs") continue;
  if (allowFiles.has(rel)) continue;
  const content = fs.readFileSync(file, "utf8");
  const badLines = content.split("\n").filter((line) => lineReadsDatabaseUrl(line));
  if (badLines.length > 0) {
    violations.push(rel);
  }
}

if (violations.length > 0) {
  console.error("Tests interdits d’accéder à DATABASE_URL — utiliser TEST_DATABASE_URL :");
  for (const v of violations) console.error(`  - ${v}`);
  process.exit(1);
}
console.log("OK — aucun test ne lit DATABASE_URL");
