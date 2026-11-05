import assert from "node:assert/strict";
import test from "node:test";
import { composeMessageBody, matchRecipient } from "./messagerie-recipients";

const people = [
  { id: "1", name: "Marie Martin", roles: ["professeur"] },
  { id: "2", name: "Paul Martin", roles: ["cpe"] },
  { id: "3", name: "Élodie Durand", roles: ["administratif"] },
];

test("titre + nom unique, accents ignorés", () => {
  const m = matchRecipient("Mme Durand", people);
  assert.equal(m.kind, "one");
  assert.equal(matchRecipient("elodie durand", people).kind, "one");
});

test("nom ambigu -> many", () => {
  const m = matchRecipient("Mme Martin", people);
  assert.equal(m.kind, "many");
  if (m.kind === "many") assert.equal(m.candidates.length, 2);
});

test("prénom + nom lève l'ambiguïté", () => {
  const m = matchRecipient("Marie Martin", people);
  assert.equal(m.kind, "one");
  if (m.kind === "one") assert.equal(m.candidate.id, "1");
});

test("inconnu / vide -> none", () => {
  assert.equal(matchRecipient("Dupont", people).kind, "none");
  assert.equal(matchRecipient("Mme", people).kind, "none");
});

test("objet placé en tête du message", () => {
  assert.equal(composeMessageBody("Réunion", "Bonjour"), "Objet : Réunion\n\nBonjour");
  assert.equal(composeMessageBody("", "Bonjour"), "Bonjour");
});
