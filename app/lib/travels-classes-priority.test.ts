import assert from "node:assert/strict";
import test from "node:test";
import {
  isTripSelectedClass,
  prioritizeClassesForTrip,
} from "./travels-classes";

test("prioritizeClassesForTrip — classes du séjour en tête", () => {
  const ordered = prioritizeClassesForTrip(
    ["1A", "2A", "3A", "3B", "3C", "4A"],
    "3A, 3C",
  );
  assert.deepEqual(ordered.slice(0, 2), ["3A", "3C"]);
  assert.deepEqual(ordered.slice(2), ["1A", "2A", "3B", "4A"]);
});

test("prioritizeClassesForTrip — match souple (3ème A ≈ 3A)", () => {
  const ordered = prioritizeClassesForTrip(["3A", "3B", "4A"], "3ème A, 3B");
  assert.equal(ordered[0], "3A");
  assert.equal(ordered[1], "3B");
  assert.equal(ordered[2], "4A");
});

test("prioritizeClassesForTrip — ne réintroduit pas de classes fantômes", () => {
  const ordered = prioritizeClassesForTrip(["3A", "3B"], "03D, 3ème 2, 3A");
  assert.deepEqual(ordered, ["3A", "3B"]);
  assert.ok(!ordered.includes("03D"));
});

test("isTripSelectedClass", () => {
  assert.equal(isTripSelectedClass("3A", "3A, 3B"), true);
  assert.equal(isTripSelectedClass("4A", "3A, 3B"), false);
});
