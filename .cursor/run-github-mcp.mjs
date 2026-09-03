#!/usr/bin/env node
/**
 * Lanceur cross-platform du MCP GitHub officiel (github-mcp-server).
 * Auth via GITHUB_PERSONAL_ACCESS_TOKEN / GITHUB_TOKEN (mcp.local.env).
 */
import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

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

if (!process.env.GITHUB_PERSONAL_ACCESS_TOKEN && process.env.GITHUB_TOKEN) {
  process.env.GITHUB_PERSONAL_ACCESS_TOKEN = process.env.GITHUB_TOKEN;
}

const token = process.env.GITHUB_PERSONAL_ACCESS_TOKEN;
if (!token) {
  console.error(
    "[github-mcp] GITHUB_PERSONAL_ACCESS_TOKEN manquant (.cursor/mcp.local.env).",
  );
  process.exit(1);
}

function candidates() {
  const home = homedir();
  return [
    process.env.GITHUB_MCP_SERVER_PATH,
    join(home, ".local", "bin", process.platform === "win32" ? "github-mcp-server.exe" : "github-mcp-server"),
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
    "[github-mcp] binaire introuvable. Télécharge github-mcp-server dans ~/.local/bin",
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
