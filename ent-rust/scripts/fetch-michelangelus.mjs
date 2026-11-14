/**
 * Télécharge Michelangelus (Microsoft / Studiogusto) pour le labo ENT.
 * Usage : node scripts/fetch-michelangelus.mjs
 * Licence : EULA Microsoft fournie dans le zip — usage local labo uniquement.
 */
import https from "https";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { execSync } from "child_process";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const dir = path.join(__dirname, "..", "crates", "ent-api", "static", "fonts");
fs.mkdirSync(dir, { recursive: true });

const ZIP_URL =
  "https://download.microsoft.com/download/e8912dd7-c7e6-4ca1-a36b-c121d3fb238f/Michelangelus%20Fonts.zip";

function get(url, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > 10) return reject(new Error("too many redirects"));
    https
      .get(url, { headers: { "User-Agent": "Mozilla/5.0 ScolIA-lab" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          get(new URL(res.headers.location, url).href, redirects + 1).then(resolve, reject);
          return;
        }
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () =>
          resolve({ status: res.statusCode, body: Buffer.concat(chunks) }),
        );
      })
      .on("error", reject);
  });
}

const zipPath = path.join(dir, "Michelangelus.zip");
const res = await get(ZIP_URL);
if (res.status !== 200 || res.body[0] !== 0x50) {
  console.error("Download failed", res.status, res.body.length);
  process.exit(1);
}
fs.writeFileSync(zipPath, res.body);
console.log("saved", res.body.length, "bytes");
execSync(
  `powershell -NoProfile -Command "Expand-Archive -Force -LiteralPath '${zipPath.replace(/'/g, "''")}' -DestinationPath '${dir.replace(/'/g, "''")}'"`,
  { stdio: "inherit" },
);
fs.unlinkSync(zipPath);
const fonts = fs.readdirSync(dir).filter((f) => /\.ttf$/i.test(f));
console.log("OK", fonts.join(", "));
