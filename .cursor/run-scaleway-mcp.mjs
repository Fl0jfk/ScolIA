#!/usr/bin/env node
/**
 * Lanceur cross-platform du MCP Scaleway (scw mcp server serve).
 */
import { spawn } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
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

/** Charge SCW_* depuis ~/.config/scw/config.yaml si `scw login` a déjà tourné. */
function loadScwConfigYaml() {
  const home = process.env.USERPROFILE || process.env.HOME || homedir();
  const configPath = join(home, ".config", "scw", "config.yaml");
  if (!existsSync(configPath)) return;
  const text = readFileSync(configPath, "utf8");
  const map = {
    "access_key": "SCW_ACCESS_KEY",
    "secret_key": "SCW_SECRET_KEY",
    "default_organization_id": "SCW_DEFAULT_ORGANIZATION_ID",
    "default_project_id": "SCW_DEFAULT_PROJECT_ID",
    "default_region": "SCW_DEFAULT_REGION",
  };
  for (const rawLine of text.split(/\r?\n/)) {
    const m = rawLine.match(/^\s*([a-z_]+):\s*(.+)\s*$/);
    if (!m) continue;
    const envKey = map[m[1]];
    if (!envKey) continue;
    let value = m[2].trim().replace(/^["']|["']$/g, "");
    if (value.startsWith("${") || value === "") continue;
    if (!process.env[envKey]) process.env[envKey] = value;
  }
}

loadScwConfigYaml();

function findInWinGetPackages() {
  const local = process.env.LOCALAPPDATA;
  if (!local) return null;
  const packages = join(local, "Microsoft", "WinGet", "Packages");
  if (!existsSync(packages)) return null;
  try {
    for (const name of readdirSync(packages)) {
      if (!name.toLowerCase().includes("scaleway")) continue;
      const exe = join(packages, name, "scaleway-cli.exe");
      if (existsSync(exe)) return exe;
    }
  } catch {
    return null;
  }
  return null;
}

function candidates() {
  const home = homedir();
  return [
    process.env.SCW_PATH,
    join(home, ".local", "bin", process.platform === "win32" ? "scw.exe" : "scw"),
    findInWinGetPackages(),
    "scw",
    "scaleway-cli",
  ].filter(Boolean);
}

function resolveScw() {
  for (const c of candidates()) {
    if (c === "scw" || c === "scaleway-cli") return c;
    if (existsSync(c)) return c;
  }
  return null;
}

const scw = resolveScw();
if (!scw) {
  console.error(
    "[scaleway-mcp] scw introuvable. Installe la CLI : winget install -e --id Scaleway.cli",
  );
  process.exit(1);
}

const required = [
  "SCW_ACCESS_KEY",
  "SCW_SECRET_KEY",
  "SCW_DEFAULT_ORGANIZATION_ID",
  "SCW_DEFAULT_PROJECT_ID",
];
const missing = required.filter((k) => !process.env[k]);
if (missing.length > 0) {
  console.error(
    `[scaleway-mcp] Variables manquantes : ${missing.join(", ")}`,
  );
  console.error(
    "[scaleway-mcp] Lance `scw login` (navigateur), ou renseigne SCW_* dans .cursor/mcp.local.env",
  );
  process.exit(1);
}

if (!process.env.SCW_DEFAULT_REGION) {
  process.env.SCW_DEFAULT_REGION = "fr-par";
}

const args = [
  "mcp",
  "server",
  "serve",
  "namespaces=container,registry,object,rdb,tem,cockpit,iam",
];

const child = spawn(scw, args, {
  stdio: "inherit",
  env: process.env,
  shell: false,
});

child.on("error", (err) => {
  console.error(`[scaleway-mcp] échec lancement (${scw}):`, err.message);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 1);
});
