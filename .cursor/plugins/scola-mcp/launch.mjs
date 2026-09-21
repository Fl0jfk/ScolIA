#!/usr/bin/env node
/**
 * Relais plugin → wrappers `.cursor/run-*-mcp.mjs` (chemins portables).
 */
import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const name = process.argv[2];
const pluginRoot = dirname(fileURLToPath(import.meta.url));
const cursorDir = join(pluginRoot, "..", "..");

const map = {
  postgres: "run-postgres-mcp.mjs",
  browser: "run-browser-mcp.mjs",
  fetch: "run-fetch-mcp.mjs",
  mistral: "run-mistral-mcp.mjs",
  scaleway: "run-scaleway-mcp.mjs",
  github: "run-github-mcp.mjs",
};

const file = map[name];
if (!file) {
  console.error(`[scola-mcp] serveur inconnu: ${name || "(vide)"}`);
  process.exit(1);
}

const child = spawn(process.execPath, [join(cursorDir, file)], {
  stdio: "inherit",
  env: process.env,
});

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 1);
});
