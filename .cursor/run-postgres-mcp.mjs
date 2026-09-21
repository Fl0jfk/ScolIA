#!/usr/bin/env node
/**
 * Lanceur cross-platform du MCP Postgres (Windows + Linux / Cloud Agents).
 * Charge MCP_DATABASE_URL depuis l'env process, puis mcp.local.env / .env.local / .env.
 */
import { spawn } from "node:child_process";
import { extendAgentPath, loadMcpEnv } from "./load-mcp-env.mjs";

extendAgentPath();
loadMcpEnv(["MCP_DATABASE_URL", "MISTRAL_API_KEY"]);

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
