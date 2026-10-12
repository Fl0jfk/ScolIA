import assert from "node:assert/strict";
import test from "node:test";
import {
  pickClassAfterMetadataResponse,
  rosterMemoryCacheKey,
  rosterMemoryWriteKeys,
} from "./stage-class-roster-panel-logic";

test("sans className — charge la première classe disponible", () => {
  const pick = pickClassAfterMetadataResponse(["3A", "3E"], "", "");
  assert.equal(pick, "3A");
});

test("sans className — conserve la classe déjà sélectionnée si toujours disponible", () => {
  const pick = pickClassAfterMetadataResponse(["3A", "3E"], "", "3E");
  assert.equal(pick, "3E");
});

test("sans className — ne relance pas si className déjà demandé", () => {
  const pick = pickClassAfterMetadataResponse(["3A"], "3A", "");
  assert.equal(pick, null);
});

test("cache mémoire — pas d’écriture sous une classe sans roster (régression M1)", () => {
  const keys = rosterMemoryWriteKeys(
    { availableClasses: ["3E"], roster: null },
    "",
  );
  assert.deepEqual(keys, []);
});

test("cache mémoire — écrit sous la classe et __default__ quand roster présent sans className", () => {
  const keys = rosterMemoryWriteKeys(
    { availableClasses: ["3E"], roster: { className: "3E" } },
    "",
  );
  assert.deepEqual(keys, [rosterMemoryCacheKey("3E"), "__default__"]);
});

test("cache mémoire — écrit uniquement la classe quand className explicite", () => {
  const keys = rosterMemoryWriteKeys(
    { availableClasses: ["3E"], roster: { className: "3E" } },
    "3E",
  );
  assert.deepEqual(keys, [rosterMemoryCacheKey("3E")]);
});
