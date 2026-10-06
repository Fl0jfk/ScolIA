/**
 * Échoue si un fichier *.test.* contourne TEST_DATABASE_URL / la garde.
 */
import fs from "node:fs";
import path from "node:path";
import {
  scanPackageJsonTestScripts,
  scanTestFileContent,
} from "./check-test-files-db-safety-core.mjs";

const root = path.resolve(import.meta.dirname, "..");

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
  if (rel === "scripts/check-test-files-db-safety.test.mjs") continue;
  if (rel === "scripts/check-test-files-db-safety-core.mjs") continue;
  const content = fs.readFileSync(file, "utf8");
  const found = scanTestFileContent(rel, content);
  if (found.length > 0) {
    violations.push({ file: rel, rules: found });
  }
}

const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const badScripts = scanPackageJsonTestScripts(pkg.scripts ?? {});
if (badScripts.length > 0) {
  violations.push({
    file: "package.json (scripts test:*)",
    rules: badScripts.map((s) => `script:${s}`),
  });
}

if (violations.length > 0) {
  console.error("Tests interdits de contourner TEST_DATABASE_URL / garde Postgres :");
  for (const v of violations) {
    console.error(`  - ${v.file}: ${v.rules.join(", ")}`);
  }
  console.error(
    "Limitation : les tests qui appellent uniquement getDb() via code applicatif ne sont pas détectés statiquement.",
  );
  process.exit(1);
}
console.log("OK — fichiers *.test.* conformes (garde TEST_DATABASE_URL)");
