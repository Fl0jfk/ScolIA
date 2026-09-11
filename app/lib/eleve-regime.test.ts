import assert from "node:assert/strict";
import test from "node:test";
import { canonicalRegimeLabel, classifyRegime, isRegimeInterne } from "./eleve-regime";

test("codes BCN / Siècle — 3 = interne, 2 = DP", () => {
  assert.equal(isRegimeInterne("3"), true);
  assert.equal(isRegimeInterne("5"), true);
  assert.equal(isRegimeInterne("Interne"), true);
  assert.equal(isRegimeInterne("2"), false);
  assert.equal(isRegimeInterne("1"), false);
  assert.equal(isRegimeInterne("0"), false);
  assert.equal(isRegimeInterne("4"), false); // interne externé = ne dort pas
  assert.equal(isRegimeInterne("Demi-pensionnaire"), false);
  assert.equal(isRegimeInterne("Externe"), false);
});

test("classifie DP et externes BCN", () => {
  assert.equal(classifyRegime("DP"), "demi_pension");
  assert.equal(classifyRegime("2"), "demi_pension");
  assert.equal(classifyRegime("6"), "demi_pension");
  assert.equal(classifyRegime("0"), "externe");
  assert.equal(classifyRegime("1"), "externe");
});

test("canonicalRegimeLabel — codes BCN", () => {
  assert.equal(canonicalRegimeLabel("2"), "Demi-pension");
  assert.equal(canonicalRegimeLabel("3"), "Interne");
  assert.equal(canonicalRegimeLabel("0"), "Externe");
  assert.equal(canonicalRegimeLabel("oui"), "Interne");
  assert.equal(canonicalRegimeLabel("x"), "Interne");
  assert.equal(canonicalRegimeLabel("non"), "Externe");
});
