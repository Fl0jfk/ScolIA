import assert from "node:assert/strict";
import test from "node:test";
import {
  filterObservedClassesForCurrentYearUi,
  type OfficialClassesResult,
} from "./nomenclature-classes";
import { foldSchoolClass } from "./school-classes-catalog";

function officialStub(locked: string[]): OfficialClassesResult {
  const canonicalByFold = new Map<string, string>();
  for (const code of locked) {
    const fold = foldSchoolClass(code);
    if (fold && !canonicalByFold.has(fold)) canonicalByFold.set(fold, code);
  }
  return {
    hasLockedSiecle: true,
    lockedClasses: [...locked].sort((a, b) => a.localeCompare(b, "fr")),
    lockedClassesByPole: { COLLÈGE: locked.filter((c) => /^[3456]/.test(c)), LYCÉE: locked.filter((c) => /^[12T]/i.test(c)) },
    classesByPole: {},
    divisions: [],
    canonicalByFold,
  };
}

test("filterObservedClassesForCurrentYearUi — garde Structures, ignore N-1 / hors étab", () => {
  const official = officialStub(["2A", "3A", "1 A"]);
  const filtered = filterObservedClassesForCurrentYearUi(
    ["2A", "3e2", "03D", "3 3", "PSA", "1 A"],
    official,
  );
  assert.deepEqual(filtered, ["1 A", "2A", "PSA"]);
});

test("filterObservedClassesForCurrentYearUi — sans Siècle, laisse passer", () => {
  const filtered = filterObservedClassesForCurrentYearUi(["3e2", "2A"], null);
  assert.deepEqual(filtered, ["2A", "3e2"]);
});
