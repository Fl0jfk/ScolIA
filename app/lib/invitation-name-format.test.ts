import assert from "node:assert/strict";
import test from "node:test";
import {
  formatInvitationEleveLabel,
  formatInvitationFirstName,
  formatInvitationLastName,
} from "./invitation-types";

test("nom de famille en majuscules", () => {
  assert.equal(formatInvitationLastName("dupont"), "DUPONT");
  assert.equal(formatInvitationLastName("de la Fontaine"), "DE LA FONTAINE");
});

test("prénom : initiale majuscule, y compris tirets", () => {
  assert.equal(formatInvitationFirstName("PIERRE"), "Pierre");
  assert.equal(formatInvitationFirstName("jean-pierre"), "Jean-Pierre");
  assert.equal(formatInvitationFirstName("léa"), "Léa");
});

test("libellé liste Prénom NOM", () => {
  assert.equal(formatInvitationEleveLabel("marie-claire", "leclerc"), "Marie-Claire LECLERC");
});
