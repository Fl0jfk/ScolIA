import assert from "node:assert/strict";
import test from "node:test";
import {
  normalizeStagePersonName,
  orphanConventionDisposition,
  stagePersonNamesMatch,
} from "./stage-person-name";

function rosterNameKey(nom: string, prenom: string): string {
  return `name:${normalizeStagePersonName(nom)}|${normalizeStagePersonName(prenom)}`;
}

test("normalizeStagePersonName — N'SONI ≡ NSONI (apostrophe)", () => {
  assert.equal(normalizeStagePersonName("N'SONI"), "nsoni");
  assert.equal(normalizeStagePersonName("NSONI"), "nsoni");
  assert.equal(normalizeStagePersonName("N’SONI"), "nsoni");
  assert.equal(normalizeStagePersonName("N`SONI"), "nsoni");
});

test("stagePersonNamesMatch — fusionne N'SONI / NSONI", () => {
  assert.equal(
    stagePersonNamesMatch(
      { nom: "N'SONI", prenom: "Dane Junior" },
      { lastName: "NSONI", firstName: "Dane Junior" },
    ),
    true,
  );
});

test("clé roster — même clé avec ou sans apostrophe", () => {
  assert.equal(rosterNameKey("N'SONI", "Dane Junior"), rosterNameKey("NSONI", "Dane Junior"));
});

test("stageStudentNameMatchesEleve — prénom composé + apostrophe", async () => {
  const { stageStudentNameMatchesEleve } = await import("./stage-person-name");
  assert.equal(
    stageStudentNameMatchesEleve("Dane Junior NSONI", {
      nom: "N'SONI",
      prenom: "Dane Junior",
    }),
    true,
  );
  assert.equal(
    stageStudentNameMatchesEleve("N'SONI Dane Junior", {
      nom: "N'SONI",
      prenom: "Dane Junior",
    }),
    true,
  );
});

test("orphanConventionDisposition — élève actif ailleurs (Seglas 2nde) → skip", () => {
  assert.equal(
    orphanConventionDisposition({ actifInOtherClass: true, knownButNotActif: false }),
    "skip_other_class",
  );
});

test("orphanConventionDisposition — sorti (Pitte) → skip", () => {
  assert.equal(
    orphanConventionDisposition({ actifInOtherClass: false, knownButNotActif: true }),
    "skip_sorti",
  );
});

test("orphanConventionDisposition — inconnu registre (possible parent) → orphelin", () => {
  assert.equal(
    orphanConventionDisposition({ actifInOtherClass: false, knownButNotActif: false }),
    "create_orphan",
  );
});
