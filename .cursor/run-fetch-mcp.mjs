#!/usr/bin/env node
/**
 * Lanceur cross-platform du MCP fetch (uvx mcp-server-fetch).
 */
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

function candidates() {
  const home = homedir();
  const local = process.env.LOCALAPPDATA || "";
  const list = [
    process.env.UVX_PATH,
    join(home, ".local", "bin", process.platform === "win32" ? "uvx.exe" : "uvx"),
    join(home, ".cargo", "bin", process.platform === "win32" ? "uvx.exe" : "uvx"),
    "uvx",
  ];
  if (local) {
    list.unshift(join(local, "uv", "bin", "uvx.exe"));
  }
  return list.filter(Boolean);
}

function resolveUvx() {
  for (const c of candidates()) {
    if (c === "uvx") return c;
    if (existsSync(c)) return c;
  }
  return null;
}

const uvx = resolveUvx();
if (!uvx) {
  console.error(
    "[fetch-mcp] uvx introuvable. Installe uv : https://docs.astral.sh/uv/getting-started/installation/",
  );
  process.exit(1);
}

const child = spawn(uvx, ["mcp-server-fetch"], {
  stdio: "inherit",
  env: process.env,
  shell: false,
});

child.on("error", (err) => {
  console.error(`[fetch-mcp] échec lancement uvx (${uvx}):`, err.message);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 1);
});
