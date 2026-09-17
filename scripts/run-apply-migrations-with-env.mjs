import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const root = path.resolve(import.meta.dirname, "..");

function loadDatabaseUrl() {
  for (const name of [".env.local", ".env"]) {
    const file = path.join(root, name);
    if (!fs.existsSync(file)) continue;
    for (const line of fs.readFileSync(file, "utf8").split(/\r?\n/)) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      const m = trimmed.match(/^DATABASE_URL\s*=\s*(.*)$/);
      if (!m) continue;
      let v = m[1].trim();
      if (
        (v.startsWith('"') && v.endsWith('"')) ||
        (v.startsWith("'") && v.endsWith("'"))
      ) {
        v = v.slice(1, -1);
      }
      return v;
    }
  }
  return process.env.DATABASE_URL || "";
}

const url = loadDatabaseUrl();
if (!url) {
  console.error("DATABASE_URL manquant (.env / .env.local).");
  process.exit(1);
}

try {
  const u = new URL(url.replace(/^postgresql:/, "postgres:"));
  console.log(`Connexion vers ${u.hostname}:${u.port || 5432} …`);
} catch {
  console.log("Connexion DATABASE_URL …");
}

const env = {
  ...process.env,
  DATABASE_URL: url,
  NODE_TLS_REJECT_UNAUTHORIZED: process.env.NODE_TLS_REJECT_UNAUTHORIZED || "0",
};

const result = spawnSync(process.execPath, ["scripts/apply-migrations-direct.mjs"], {
  cwd: root,
  env,
  encoding: "utf8",
  stdio: "inherit",
});

process.exit(result.status ?? 1);
