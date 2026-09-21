/**
 * Charge les env MCP depuis mcp.local.env / .env.local / .env
 * sans écraser les secrets déjà injectés par le process (dashboard Cloud).
 */
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const cursorDir = dirname(fileURLToPath(import.meta.url));
export const repoRoot = join(cursorDir, "..");

export function loadEnvFile(filePath) {
  if (!existsSync(filePath)) return;
  const text = readFileSync(filePath, "utf8");
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    const eq = line.indexOf("=");
    if (eq <= 0) continue;
    const key = line.slice(0, eq).trim();
    let value = line.slice(eq + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!(key in process.env) || process.env[key] === "") {
      process.env[key] = value;
    }
  }
}

/**
 * @param {string[]} preservedKeys clés déjà présentes dans process.env à restaurer
 */
export function loadMcpEnv(preservedKeys = []) {
  const preserved = {};
  for (const key of preservedKeys) {
    if (process.env[key]) preserved[key] = process.env[key];
  }
  loadEnvFile(join(repoRoot, ".cursor", "mcp.local.env"));
  loadEnvFile(join(repoRoot, ".env.local"));
  loadEnvFile(join(repoRoot, ".env"));
  for (const [key, value] of Object.entries(preserved)) {
    process.env[key] = value;
  }
}

export function extendAgentPath() {
  const home = process.env.HOME || process.env.USERPROFILE || "";
  const extras = [
    join(home, ".local", "bin"),
    join(home, "bin"),
    "/usr/local/bin",
  ];
  const current = (process.env.PATH || "").split(":").filter(Boolean);
  const merged = [...extras.filter((p) => p && existsSync(p)), ...current];
  process.env.PATH = [...new Set(merged)].join(":");
}
