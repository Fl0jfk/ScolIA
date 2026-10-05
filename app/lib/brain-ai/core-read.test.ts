import assert from "node:assert/strict";
import test from "node:test";
import {
  canSeeFamille,
  eleveAllowedByClassRestriction,
  explainOccupancyTag,
  professorClassRestriction,
  shapeEleveForRoles,
} from "./core-read";

const eleve = {
  id: "e1",
  ine: "INE123",
  nom: "DUPONT",
  prenom: "Thomas",
  classe: "4B",
  regime: "DP",
  status: "inscrit",
  parentEmail: "parent@example.com",
  parent1Phone: "0600000000",
};

test("accueil — liste sans foyer", () => {
  assert.equal(canSeeFamille(["accueil"], false), false);
  const shaped = shapeEleveForRoles(eleve, ["accueil"], false);
  assert.equal(shaped.nom, "DUPONT");
  assert.equal(shaped.classe, "4B");
  assert.equal(shaped.parentEmail, undefined);
  assert.equal(shaped.ine, null);
});

test("direction — foyer visible", () => {
  assert.equal(canSeeFamille(["direction_college"], false), true);
  const shaped = shapeEleveForRoles(eleve, ["direction_college"], false);
  assert.equal(shaped.parentEmail, "parent@example.com");
  assert.equal(shaped.ine, "INE123");
});

test("professeur — restriction classe ; hors classe refusé", () => {
  const restriction = professorClassRestriction(["professeur"], false);
  assert.ok(Array.isArray(restriction));
  assert.equal(eleveAllowedByClassRestriction("4B", ["4B", "4A"]), true);
  assert.equal(eleveAllowedByClassRestriction("3A", ["4B"]), false);
  assert.equal(eleveAllowedByClassRestriction("4B", null), true);
});

test("en_sortie — expliqué comme sortie, pas absence bulletin", () => {
  const text = explainOccupancyTag("en_sortie");
  assert.match(text, /sortie/i);
  assert.match(text, /pas une absence bulletin/i);
  assert.doesNotMatch(text, /malade|justif/i);
});

test("a_infirmerie — expliqué, pas absence bulletin", () => {
  const text = explainOccupancyTag("a_infirmerie");
  assert.match(text, /infirmerie/i);
  assert.match(text, /pas une absence bulletin/i);
});

test("hors_etablissement — dernier passage portail", () => {
  const text = explainOccupancyTag("hors_etablissement");
  assert.match(text, /portail/i);
});
