import assert from "node:assert/strict";
import test from "node:test";
import { isInternatEveningRollCallDay, parisWeekdayFromDateKey } from "./internat-stats";

/** 2026-09-07 = lundi … 2026-09-13 = dimanche */
test("appel du soir internat : lun–jeu uniquement", () => {
  assert.equal(parisWeekdayFromDateKey("2026-09-07"), 1);
  assert.equal(parisWeekdayFromDateKey("2026-09-11"), 5);
  assert.equal(isInternatEveningRollCallDay("2026-09-07"), true); // lun
  assert.equal(isInternatEveningRollCallDay("2026-09-08"), true); // mar
  assert.equal(isInternatEveningRollCallDay("2026-09-09"), true); // mer
  assert.equal(isInternatEveningRollCallDay("2026-09-10"), true); // jeu
  assert.equal(isInternatEveningRollCallDay("2026-09-11"), false); // ven
  assert.equal(isInternatEveningRollCallDay("2026-09-12"), false); // sam
  assert.equal(isInternatEveningRollCallDay("2026-09-13"), false); // dim
});
