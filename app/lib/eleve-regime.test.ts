import assert from "node:assert/strict";
import test from "node:test";
import { classifyRegime, isRegimeInterne } from "./eleve-regime";
import { parsePhotoFilename } from "./eleve-photos-match";

test("détecte les codes Siècle internes", () => {
  assert.equal(isRegimeInterne("2"), true);
  assert.equal(isRegimeInterne("3"), true);
  assert.equal(isRegimeInterne("Interne"), true);
  assert.equal(isRegimeInterne("INTERNE-EXTERNE"), true);
  assert.equal(isRegimeInterne("1"), false);
  assert.equal(isRegimeInterne("0"), false);
  assert.equal(isRegimeInterne("Demi-pensionnaire"), false);
  assert.equal(isRegimeInterne("Externe"), false);
});

test("classifie DP", () => {
  assert.equal(classifyRegime("DP"), "demi_pension");
  assert.equal(classifyRegime("1"), "demi_pension");
});

test("parse NOM Prenom photo filename", () => {
  assert.deepEqual(parsePhotoFilename("DUPONT Marie.jpg"), { nom: "DUPONT", prenom: "Marie" });
  assert.deepEqual(parsePhotoFilename("MARTIN_Jean-Pierre.png"), {
    nom: "MARTIN",
    prenom: "Jean Pierre",
  });
});
