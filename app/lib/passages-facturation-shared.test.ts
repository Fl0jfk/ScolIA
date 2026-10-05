import assert from "node:assert/strict";
import test from "node:test";
import {
  eachIsoDateInclusive,
  isCantineFactuMode,
  roundMoney,
} from "@/app/lib/passages-facturation-shared";

test("eachIsoDateInclusive", () => {
  assert.deepEqual(eachIsoDateInclusive("2026-09-22", "2026-09-22"), ["2026-09-22"]);
  assert.deepEqual(eachIsoDateInclusive("2026-09-21", "2026-09-23"), [
    "2026-09-21",
    "2026-09-22",
    "2026-09-23",
  ]);
  assert.deepEqual(eachIsoDateInclusive("2026-09-23", "2026-09-21"), []);
});

test("isCantineFactuMode + roundMoney", () => {
  assert.equal(isCantineFactuMode("forfait"), true);
  assert.equal(isCantineFactuMode("reel"), true);
  assert.equal(isCantineFactuMode("mixte"), false);
  assert.equal(roundMoney(5.005), 5.01);
  assert.equal(roundMoney(2 * 5), 10);
});
