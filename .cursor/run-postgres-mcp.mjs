#!/usr/bin/env node
/**
 * Lanceur cross-platform du MCP Postgres (Windows + Linux / Cloud Agents).
 * Charge MCP_DATABASE_URL depuis l'env process, puis mcp.local.env / .env.local / .env.
 */
import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const preserved = {
  MCP_DATABASE_URL: process.env.MCP_DATABASE_URL,
  MISTRAL_API_KEY: process.env.MISTRAL_API_KEY,
};

function loadEnvFile(filePath) {
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

loadEnvFile(join(root, ".cursor", "mcp.local.env"));
loadEnvFile(join(root, ".env.local"));
loadEnvFile(join(root, ".env"));

if (preserved.MCP_DATABASE_URL) {
  process.env.MCP_DATABASE_URL = preserved.MCP_DATABASE_URL;
}
if (preserved.MISTRAL_API_KEY) {
  process.env.MISTRAL_API_KEY = preserved.MISTRAL_API_KEY;
}

if (!process.env.MCP_DATABASE_URL && process.env.DATABASE_URL) {
  process.env.MCP_DATABASE_URL = process.env.DATABASE_URL;
}

const url = process.env.MCP_DATABASE_URL;
if (!url) {
  console.error(
    "[postgres-mcp] Définis MCP_DATABASE_URL (.cursor/mcp.local.env) ou DATABASE_URL (.env / .env.local).",
  );
  process.exit(1);
}

const target =
  url.includes("127.0.0.1") || url.includes("localhost") ? "local" : "remote";
console.error(`[postgres-mcp] target=${target}`);

const child = spawn(
  "npx",
  ["-y", "@modelcontextprotocol/server-postgres@0.6.2", url],
  {
    stdio: "inherit",
    env: process.env,
    shell: process.platform === "win32",
  },
);

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 1);
});
