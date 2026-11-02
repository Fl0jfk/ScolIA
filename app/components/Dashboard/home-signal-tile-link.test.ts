import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { HOME_SIGNAL_TILE_LINK_CLASS } from "@/app/components/Dashboard/home-signal-tile-link";

const here = dirname(fileURLToPath(import.meta.url));

test("HOME_SIGNAL_TILE_LINK_CLASS — couvre toute la tuile (inset-0)", () => {
  assert.match(HOME_SIGNAL_TILE_LINK_CLASS, /absolute inset-0/);
});

test("HomeSignalBento — utilise le lien étendu sur la tuile", () => {
  const src = readFileSync(join(here, "HomeSignalBento.tsx"), "utf8");
  assert.match(src, /HOME_SIGNAL_TILE_LINK_CLASS/);
  assert.match(src, /<Link[\s\S]*?className=\{HOME_SIGNAL_TILE_LINK_CLASS\}/);
});
