import assert from "node:assert/strict";
import test from "node:test";
import { parseTravelDayStartMs } from "./travels-date-parse";

test("parseTravelDayStartMs — ISO et formats FR", () => {
  const iso = parseTravelDayStartMs("2026-10-14");
  const fr = parseTravelDayStartMs("14/10/2026");
  assert.equal(iso, fr);
  assert.ok(iso != null);
});

test("parseTravelDayStartMs — invalide", () => {
  assert.equal(parseTravelDayStartMs(""), null);
  assert.equal(parseTravelDayStartMs("pas-une-date"), null);
});
