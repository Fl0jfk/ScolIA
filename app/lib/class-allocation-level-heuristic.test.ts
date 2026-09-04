import assert from "node:assert/strict";
import test from "node:test";
import { guessClassLevelFromClasse } from "./class-allocation-level-heuristic";
import { resolveEleveCycle } from "./accueil-absences-types";

const cases: Array<[string, "ecole" | "college" | "lycee"]> = [
  ["JE1  MME  BAYEL", "ecole"],
  ["JE2  MME CARTIER", "ecole"],
  ["JE3   MME DOUGHTY", "ecole"],
  ["JE4  MME LOURDEL", "ecole"],
  ["CP  MME PICHURON", "ecole"],
  ["CPA", "ecole"],
  ["CE1B", "ecole"],
  ["TPS", "ecole"],
  ["3°A", "college"],
  ["5°B", "college"],
  ["5°A", "college"],
  ["6°F", "college"],
  ["3B", "college"],
  ["3 F", "college"],
  ["1C", "lycee"],
  ["2A", "lycee"],
  ["TA", "lycee"],
];

test("guessClassLevelFromClasse reconnaît les libellés Providence / SIECLE", () => {
  for (const [classe, expected] of cases) {
    assert.equal(
      guessClassLevelFromClasse(classe),
      expected,
      `${classe} → ${expected}`,
    );
  }
});

test("resolveEleveCycle privilégie la classe quand le secteur est vide ou faux", () => {
  assert.equal(
    resolveEleveCycle({ secteur: null, classe: "JE3   MME DOUGHTY" }),
    "ecole",
  );
  assert.equal(resolveEleveCycle({ secteur: null, classe: "5°B" }), "college");
  assert.equal(
    resolveEleveCycle({ secteur: "lycee", classe: "JE4  MME LOURDEL" }),
    "ecole",
  );
});
