#!/usr/bin/env node
/**
 * Lanceur MCP Mistral. Clé : MISTRAL_API_KEY (env / mcp.local.env / .env.local).
 */
import { spawn } from "node:child_process";
import { extendAgentPath, loadMcpEnv } from "./load-mcp-env.mjs";

extendAgentPath();
loadMcpEnv(["MISTRAL_API_KEY"]);

if (!process.env.MISTRAL_API_KEY) {
  console.error(
    "[mistral-mcp] MISTRAL_API_KEY manquant (process.env, .cursor/mcp.local.env ou .env.local).",
  );
  process.exit(1);
}

const child = spawn("npx", ["-y", "mistral-mcp@0.8.2"], {
  stdio: "inherit",
  env: process.env,
  shell: process.platform === "win32",
});

child.on("error", (err) => {
  console.error("[mistral-mcp] échec lancement:", err.message);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 1);
});
