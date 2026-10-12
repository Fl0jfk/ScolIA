import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  formatSstFicheCreatedSuccess,
  sstFicheCreatedSuccessChannel,
} from "./fiche-submit-feedback";

describe("formatSstFicheCreatedSuccess", () => {
  it("formate le numéro de fiche", () => {
    assert.equal(formatSstFicheCreatedSuccess(12), "Fiche n°12 enregistrée.");
    assert.equal(formatSstFicheCreatedSuccess(undefined), "Fiche n°— enregistrée.");
  });
});

describe("sstFicheCreatedSuccessChannel (M1)", () => {
  it("après retour accueil, le succès va sur le bandeau page (role=status)", () => {
    assert.equal(sstFicheCreatedSuccessChannel("accueil"), "page");
    assert.equal(sstFicheCreatedSuccessChannel("suivi"), "page");
  });

  it("reste sur le formulaire uniquement si l’onglet fiche est encore actif", () => {
    assert.equal(sstFicheCreatedSuccessChannel("fiche"), "form");
  });
});
