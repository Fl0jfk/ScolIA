import assert from "node:assert/strict";
import test from "node:test";
import type { TravelsTrip } from "./travels-types";
import {
  comptaDefinitiveCostPerStudent,
  computeComptaSheetDerived,
  emptyComptaSheet,
  ensureManualDepenseSources,
  filterComptaDepenses,
  newManualDepenseLine,
  resolveComptaDepenses,
} from "./travels-compta-sheet";

function stubTrip(overrides?: Partial<TravelsTrip>): TravelsTrip {
  return {
    id: "trip-test",
    type: "SIMPLE",
    status: "EN_ATTENTE_COMPTA",
    ownerId: "u1",
    ownerName: "Test",
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    data: {
      title: "Sortie test",
      classes: "4B",
      ...(overrides?.data || {}),
    },
    receivedDevis: [],
    ...overrides,
  } as TravelsTrip;
}

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

test("prix annoncé prime sur le coût calculé pour le définitif et les recettes", () => {
  // Coût prévisionnel arrondi = 27 €, mais prix déjà annoncé = 28 €
  const sheet = computeComptaSheetDerived({
    ...emptyComptaSheet(),
    nbEleves: 10,
    depenses: [{ label: "Total", amount: 250 }],
    margeSecuriteEuro: 20,
    recettesElevesFigees: true,
    prixParEleveAnnonce: 28,
    nbElevesFactures: 10,
    margeFigeeEuro: 20,
  });

  assert.equal(sheet.coutPrevisionnelParEleve, 27);
  assert.equal(sheet.prixParEleveAvecSubventions, 28);
  assert.equal(comptaDefinitiveCostPerStudent(sheet), 28);
  assert.equal(sheet.recettesEleves, 280);
  assert.equal(sheet.totalRecettes, 280);
  assert.equal(sheet.excedentOuDeficit, 30);
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

test("ligne de dépense manuelle (y compris libellé transport) reste après filtre", () => {
  const trip = stubTrip();
  const manual = newManualDepenseLine({ label: "Transport complémentaire taxi", amount: 120 });
  const filtered = filterComptaDepenses(
    [{ label: "Transport (devis bus signé)", amount: 800, source: "devis_signe" }, manual],
    trip,
  );
  assert.ok(filtered.some((l) => l.source === manual.source && l.amount === 120));
});

test("resolveComptaDepenses conserve les lignes manuel: au rechargement", () => {
  const trip = stubTrip({
    data: {
      title: "Sortie",
      classes: "4B",
      signedQuoteUrl: "https://example.com/devis.pdf",
      selectedBusQuote: { providerName: "Autocars Dupont", fileUrl: "https://example.com/devis.pdf", price: "800" },
    },
  } as Partial<TravelsTrip>);
  const manual = newManualDepenseLine({ label: "Entrées musée", amount: 210 });
  const resolved = resolveComptaDepenses(trip, [
    { label: "Transport (devis bus signé)", amount: 800, source: "devis_signe" },
    manual,
  ]);
  assert.ok(resolved.some((l) => l.source === manual.source && l.amount === 210));
});

test("ensureManualDepenseSources tamponne les lignes sans source", () => {
  const stamped = ensureManualDepenseSources([
    { label: "Hébergement", amount: 400 },
    { label: "Transport (devis bus signé)", amount: 800, source: "devis_signe" },
    { label: "", amount: null },
  ]);
  assert.ok(String(stamped[0].source || "").startsWith("manuel:"));
  assert.equal(stamped[1].source, "devis_signe");
  assert.equal(stamped[2].source == null || stamped[2].source === "", true);
});
