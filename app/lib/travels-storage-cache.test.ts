import assert from "node:assert/strict";
import test from "node:test";
import { travelsIndexCacheHitUsable } from "./travels-index-cache";
import type { TravelsTrip } from "./travels-types";

const sample: TravelsTrip[] = [
  { id: "t1", type: "SIMPLE", status: "VALIDE", data: { title: "x" } },
];

test("travelsIndexCacheHitUsable — refuse tableau vide", () => {
  assert.equal(travelsIndexCacheHitUsable([]), false);
});

test("travelsIndexCacheHitUsable — accepte index non vide", () => {
  assert.equal(travelsIndexCacheHitUsable(sample), true);
});

test("travelsIndexCacheHitUsable — refuse null / objet", () => {
  assert.equal(travelsIndexCacheHitUsable(null), false);
  assert.equal(travelsIndexCacheHitUsable({}), false);
});
