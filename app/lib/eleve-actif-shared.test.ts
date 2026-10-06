import assert from "node:assert/strict";
import test from "node:test";
import { isEleveActifPourListes, isEleveSortantEtablissement } from "./eleve-actif-shared";
import { isEleveScolarise } from "./eleves-config";

/** 6 oct. 2026 à midi Paris (indépendant du fuseau de la VM). */
const now = new Date("2026-10-06T12:00:00+02:00");

test("élève inscrit sans date de sortie → actif listes", () => {
  assert.equal(isEleveActifPourListes({ status: "inscrit" }, now), true);
  assert.equal(isEleveScolarise({ status: "inscrit" }), true);
});

test("date de sortie passée → hors listes ; jour J encore actif", () => {
  assert.equal(isEleveActifPourListes({ status: "inscrit", dateSortie: "2026-10-05" }, now), false);
  assert.equal(isEleveActifPourListes({ status: "inscrit", dateSortie: "2026-10-06" }, now), true);
  assert.equal(isEleveActifPourListes({ status: "inscrit", dateSortie: "2026-10-07" }, now), true);
  assert.equal(isEleveSortantEtablissement({ status: "inscrit", dateSortie: "2026-10-06" }, now), false);
});

test("statut ancien → hors listes même sans date", () => {
  assert.equal(isEleveActifPourListes({ status: "ancien" }, now), false);
  assert.equal(isEleveSortantEtablissement({ status: "ancien" }, now), true);
});
