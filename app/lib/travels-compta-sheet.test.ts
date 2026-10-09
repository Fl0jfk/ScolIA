import assert from "node:assert/strict";
import test from "node:test";
import { computeComptaSheetDerived, emptyComptaSheet } from "./travels-compta-sheet";

test("recettes élèves = prix à l'euro près × nb (pas dépenses + marge exact)", () => {
  // Dépenses 798 €, marge 5 % = 39,90 € → (798+39,90)/57 = 14,70 € → ceil = 15 €
  // Recettes élèves attendues : 15 × 57 = 855 € (excédent 57 €, pas 39,90 €)
  const sheet = computeComptaSheetDerived({
    ...emptyComptaSheet(),
    nbEleves: 57,
    depenses: [{ label: "Total", amount: 798 }],
    margeSecuriteEuro: 39.9,
  });

  assert.equal(sheet.margeRisqueMontant, 39.9);
  assert.equal(sheet.montantCibleFacturation, 837.9);
  assert.equal(sheet.prixParEleveAvecSubventions, 15);
  assert.equal(sheet.recettesEleves, 855);
  assert.equal(sheet.totalRecettes, 855);
  assert.equal(sheet.excedentOuDeficit, 57);
  assert.equal(sheet.facturations[0]?.montant, 855);
});

test("recettes élèves figées restent sur prix annoncé × nb facturés", () => {
  const sheet = computeComptaSheetDerived({
    ...emptyComptaSheet(),
    nbEleves: 57,
    depenses: [{ label: "Total", amount: 798 }],
    margeSecuriteEuro: 39.9,
    recettesElevesFigees: true,
    prixParEleveAnnonce: 15,
    nbElevesFactures: 57,
    margeFigeeEuro: 39.9,
  });

  assert.equal(sheet.recettesEleves, 855);
  assert.equal(sheet.excedentOuDeficit, 57);
});

test("avec subventions, recettes élèves = ceil((D+M−S)/n) × n", () => {
  // D=798, M=39,90, S=57 → (837,90−57)/57 = 13,70 → ceil 14 → 14×57 = 798
  const sheet = computeComptaSheetDerived({
    ...emptyComptaSheet(),
    nbEleves: 57,
    depenses: [{ label: "Total", amount: 798 }],
    margeSecuriteEuro: 39.9,
    recettesLignes: [{ label: "APEL", amount: 57 }],
  });

  assert.equal(sheet.prixParEleveAvecSubventions, 14);
  assert.equal(sheet.recettesEleves, 798);
  assert.equal(sheet.totalRecettes, 855);
  assert.equal(sheet.excedentOuDeficit, 57);
});
