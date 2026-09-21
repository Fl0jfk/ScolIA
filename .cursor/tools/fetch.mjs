#!/usr/bin/env node
/**
 * Repli fetch sans MCP : GET une URL, imprime le texte (HTML réduit).
 * Usage : node .cursor/tools/fetch.mjs <url>
 */
const url = process.argv[2];
if (!url || !/^https?:\/\//i.test(url)) {
  console.error("Usage: node .cursor/tools/fetch.mjs <http(s)://url>");
  process.exit(1);
}

const res = await fetch(url, {
  redirect: "follow",
  headers: { "user-agent": "ScolIA-agent-fetch/1.0" },
});
const ctype = res.headers.get("content-type") || "";
const body = await res.text();
process.stderr.write(`[tools/fetch] ${res.status} ${ctype.split(";")[0]}\n`);
if (!res.ok) {
  process.stdout.write(body.slice(0, 4000));
  process.exit(1);
}
if (ctype.includes("html")) {
  const text = body
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  process.stdout.write(text.slice(0, 20_000) + (text.length > 20_000 ? "\n…" : "") + "\n");
} else {
  process.stdout.write(body.slice(0, 20_000) + (body.length > 20_000 ? "\n…" : "") + "\n");
}
