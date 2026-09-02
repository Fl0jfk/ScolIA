import { test } from "node:test";
import assert from "node:assert/strict";
import { canSeeAccueilBoardKind } from "./accueil-absences-access";

test("CPE voit élèves et profs, pas le personnel OGEC", () => {
  assert.equal(canSeeAccueilBoardKind("eleve", ["cpe"]), true);
  assert.equal(canSeeAccueilBoardKind("professeur", ["cpe"]), true);
  assert.equal(canSeeAccueilBoardKind("ogec", ["cpe"]), false);
});

test("compta et direction voient l’OGEC", () => {
  assert.equal(canSeeAccueilBoardKind("ogec", ["comptabilite"]), true);
  assert.equal(canSeeAccueilBoardKind("ogec", ["direction_lycee"]), true);
  assert.equal(canSeeAccueilBoardKind("ogec", ["surveillant"]), false);
});

test("admin établissement voit l’OGEC", () => {
  assert.equal(canSeeAccueilBoardKind("ogec", ["admin"]), true);
});

test("rôle accueil voit l’OGEC (sinon board vide après déclaration)", () => {
  assert.equal(canSeeAccueilBoardKind("ogec", ["accueil"]), true);
  assert.equal(canSeeAccueilBoardKind("eleve", ["accueil"]), true);
  assert.equal(canSeeAccueilBoardKind("professeur", ["accueil"]), true);
});
