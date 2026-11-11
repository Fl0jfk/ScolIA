import assert from "node:assert/strict";
import test from "node:test";
import { safeSecretEqual } from "@/app/lib/safe-secret-equal";

test("NJ-52 : comparaison de secrets", () => {
  assert.equal(safeSecretEqual("abc", "abc"), true);
  assert.equal(safeSecretEqual("abc", "abd"), false);
  assert.equal(safeSecretEqual("abc", "abcd"), false);
  assert.equal(safeSecretEqual("", ""), false);
  assert.equal(safeSecretEqual(undefined, "x"), false);
});
