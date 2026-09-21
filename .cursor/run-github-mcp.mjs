#!/usr/bin/env node
/**
 * Lanceur cross-platform du MCP GitHub officiel (github-mcp-server).
 * Auth via GITHUB_PERSONAL_ACCESS_TOKEN / GITHUB_TOKEN (env / mcp.local.env).
 * Pas d’OAuth interactif.
 */
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { extendAgentPath, loadMcpEnv } from "./load-mcp-env.mjs";

extendAgentPath();
loadMcpEnv(["GITHUB_PERSONAL_ACCESS_TOKEN", "GITHUB_TOKEN"]);

if (!process.env.GITHUB_PERSONAL_ACCESS_TOKEN && process.env.GITHUB_TOKEN) {
  process.env.GITHUB_PERSONAL_ACCESS_TOKEN = process.env.GITHUB_TOKEN;
}

const token = process.env.GITHUB_PERSONAL_ACCESS_TOKEN;
if (!token) {
  console.error(
    "[github-mcp] GITHUB_PERSONAL_ACCESS_TOKEN / GITHUB_TOKEN manquant — serveur optionnel, ignoré.",
  );
  process.exit(1);
}

function candidates() {
  const home = homedir();
  return [
    process.env.GITHUB_MCP_SERVER_PATH,
    "/usr/local/bin/github-mcp-server",
    join(
      home,
      ".local",
      "bin",
      process.platform === "win32" ? "github-mcp-server.exe" : "github-mcp-server",
    ),
    join(
      home,
      "bin",
      process.platform === "win32" ? "github-mcp-server.exe" : "github-mcp-server",
    ),
    "github-mcp-server",
  ].filter(Boolean);
}

function resolveBinary() {
  for (const c of candidates()) {
    if (c === "github-mcp-server") return c;
    if (existsSync(c)) return c;
  }
  return null;
}

const bin = resolveBinary();
if (!bin) {
  console.error(
    "[github-mcp] binaire introuvable. Relance .cursor/install.sh.",
  );
  process.exit(1);
}

const child = spawn(bin, ["stdio"], {
  stdio: "inherit",
  env: process.env,
  shell: false,
});

child.on("error", (err) => {
  console.error(`[github-mcp] échec lancement (${bin}):`, err.message);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 1);
});
