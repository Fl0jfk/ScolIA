#!/usr/bin/env node
/**
 * Bootstrap labo hors prod : schéma (drizzle push) + seed Leo.
 * Ne touche jamais main / prod. URL = locale ou labo (assert-lab-database).
 *
 * Usage :
 *   SCOLA_ENV=lab ALLOW_LAB_MIGRATION=1 DATABASE_URL=… npm run seed:labo
 */
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { assertLabOrLocalDatabase } from "../assert-lab-database.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
process.chdir(root);

const url = process.env.DATABASE_URL?.trim();
if (!url) {
  console.error("[labo] DATABASE_URL manquante");
  process.exit(1);
}

assertLabOrLocalDatabase(url, { action: "bootstrap labo (push + seed)" });

process.env.SCOLA_ENV = process.env.SCOLA_ENV || "lab";
process.env.NEXT_PUBLIC_SCOLA_ENV = process.env.NEXT_PUBLIC_SCOLA_ENV || "lab";

function run(cmd, args) {
  console.log(`[labo] $ ${cmd} ${args.join(" ")}`);
  const r = spawnSync(cmd, args, {
    stdio: "inherit",
    env: process.env,
    shell: false,
  });
  if (r.status !== 0) {
    process.exit(r.status ?? 1);
  }
}

run("npx", ["drizzle-kit", "push", "--force"]);
run("npm", ["run", "seed:dev"]);

console.log("");
console.log("[labo] OK — seed Leo prêt.");
console.log("[labo] Staff  : admin@localhost.dev / DevLocalPass1!  (TOTP: npm run seed:dev:totp)");
console.log("[labo] Parent : parent@localhost.dev / DevParentPass1!  → /quotidien");
console.log("[labo] Élève  : Leo JUSTIF (4B)");
