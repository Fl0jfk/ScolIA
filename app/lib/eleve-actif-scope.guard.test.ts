import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const ROOT = path.join(import.meta.dirname, "..", "..");

function walkTsFiles(dir: string, out: string[] = []): string[] {
  for (const name of fs.readdirSync(dir)) {
    if (name === "node_modules" || name === ".next") continue;
    const full = path.join(dir, name);
    const st = fs.statSync(full);
    if (st.isDirectory()) {
      walkTsFiles(full, out);
    } else if (name.endsWith(".ts") && !name.endsWith(".test.ts")) {
      out.push(full);
    }
  }
  return out;
}

test("aucune requête liste ne filtre uniquement eq(eleve.status, inscrit)", () => {
  const offenders: string[] = [];
  const scanRoots = [path.join(ROOT, "app", "lib"), path.join(ROOT, "app", "api")];
  for (const root of scanRoots) {
    for (const file of walkTsFiles(root)) {
      if (file.includes("eleve-actif-scope.ts")) continue;
      const rel = path.relative(ROOT, file);
      const content = fs.readFileSync(file, "utf8");
      if (content.includes('eq(eleve.status, "inscrit")')) {
        offenders.push(rel);
      }
    }
  }
  assert.deepEqual(
    offenders,
    [],
    `Utiliser drizzleEleveActifPourListes() : ${offenders.join(", ")}`,
  );
});
