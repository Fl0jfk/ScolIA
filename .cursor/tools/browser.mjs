#!/usr/bin/env node
/**
 * Repli browser sans MCP : screenshot Chromium via Playwright CLI.
 * Usage : node .cursor/tools/browser.mjs <url> [screenshot.png]
 */
import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";

const url = process.argv[2];
const outArg = process.argv[3];
if (!url || !/^https?:\/\//i.test(url)) {
  console.error(
    "Usage: node .cursor/tools/browser.mjs <http(s)://url> [screenshot.png]",
  );
  process.exit(1);
}

const home = process.env.HOME || homedir();
process.env.PATH = [
  join(home, ".local", "bin"),
  join(home, "bin"),
  "/usr/local/bin",
  process.env.PATH || "",
].join(":");

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const out =
  outArg || join(repoRoot, ".cursor", "tools", ".last-screenshot.png");
mkdirSync(dirname(out), { recursive: true });

const child = spawn(
  "npx",
  [
    "-y",
    "playwright",
    "screenshot",
    "--full-page",
    "--browser=chromium",
    "--ignore-https-errors",
    url,
    out,
  ],
  { stdio: "inherit", env: process.env },
);

child.on("error", (err) => {
  console.error("[tools/browser] playwright screenshot:", err.message);
  process.exit(1);
});

child.on("exit", (code) => {
  if (code === 0) {
    process.stdout.write(JSON.stringify({ url, screenshot: out }) + "\n");
  }
  process.exit(code ?? 1);
});
