import { readFileSync, existsSync } from "node:fs";
import { defineConfig } from "drizzle-kit";
import { validateDevScriptDatabaseUrl } from "./scripts/test-database-guard.mjs";

function loadEnvFile(path: string) {
  if (!existsSync(path)) return;
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq <= 0) continue;
    const key = trimmed.slice(0, eq);
    let value = trimmed.slice(eq + 1);
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env)) process.env[key] = value;
  }
}

loadEnvFile(".env.local");
loadEnvFile(".env");

const drizzleKitCommand = process.argv[2] ?? "";
const guardedDrizzleCommands = new Set(["push", "drop", "migrate"]);
if (guardedDrizzleCommands.has(drizzleKitCommand)) {
  const guard = validateDevScriptDatabaseUrl(process.env.DATABASE_URL);
  if (!guard.ok) {
    console.error(`[db-guard] ${guard.reason}`);
    console.error(
      "[db-guard] drizzle-kit push/drop/migrate limités à une base locale de dev/test.",
    );
    process.exit(1);
  }
}

export default defineConfig({
  schema: "./db/schema.ts",
  out: "./drizzle",
  dialect: "postgresql",
  dbCredentials: {
    url: process.env.DATABASE_URL ?? "postgresql://localhost:5432/scola",
  },
});
