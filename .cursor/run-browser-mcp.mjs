#!/usr/bin/env node
/**
 * Lanceur MCP Playwright (browser). Chromium installé par install.sh.
 */
import { spawn } from "node:child_process";
import { extendAgentPath } from "./load-mcp-env.mjs";

extendAgentPath();

const child = spawn(
  "npx",
  ["-y", "@playwright/mcp@0.0.79"],
  {
    stdio: "inherit",
    env: process.env,
    shell: process.platform === "win32",
  },
);

child.on("error", (err) => {
  console.error("[browser-mcp] échec lancement Playwright MCP:", err.message);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 1);
});
