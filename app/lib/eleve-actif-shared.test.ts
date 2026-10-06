import assert from "node:assert/strict";
import test from "node:test";
import { isEleveActifPourListes, isEleveSortantEtablissement } from "./eleve-actif-shared";
import { isEleveScolarise } from "./eleves-config";

const now = new Date(2026, 9, 6); // 6 oct. 2026

test("élève inscrit sans date de sortie → actif listes", () => {
  assert.equal(isEleveActifPourListes({ status: "inscrit" }, now), true);
  assert.equal(isEleveScolarise({ status: "inscrit" }), true);
});

test("date de sortie passée ou jour J → hors listes, sortant au dossier", () => {
  assert.equal(isEleveActifPourListes({ status: "inscrit", dateSortie: "2026-10-05" }, now), false);
  assert.equal(isEleveActifPourListes({ status: "inscrit", dateSortie: "2026-10-06" }, now), false);
  assert.equal(isEleveActifPourListes({ status: "inscrit", dateSortie: "2026-10-07" }, now), true);
  assert.equal(isEleveSortantEtablissement({ status: "inscrit", dateSortie: "2026-10-06" }, now), true);
});

test("statut ancien → hors listes même sans date", () => {
  assert.equal(isEleveActifPourListes({ status: "ancien" }, now), false);
  assert.equal(isEleveSortantEtablissement({ status: "ancien" }, now), true);
});
