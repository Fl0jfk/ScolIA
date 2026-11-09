import { test } from "node:test";
import assert from "node:assert/strict";
import { looksLikeStaffAbsenceRequest } from "./request-absence-detect";

const yes = [
  "Demande d'autorisation d'absence pour le 14 novembre",
  "Bonjour, je serai absente jeudi matin pour un rendez-vous médical",
  "Je dois m’absenter le 20/11 de 10h à 12h",
  "Absence du 3 au 5 décembre (obsèques)",
  "Mon enfant malade, je ne pourrai pas venir demain",
  "Je souhaiterais poser une demi-journée de congé",
  "Congé exceptionnel pour mariage",
  "Arrêt maladie à partir de lundi",
  "AUTORISATION D ABSENCE",
];

const no = [
  "La porte de la salle B12 ne ferme plus",
  "Absence de chauffage dans le bâtiment C",
  "Il faudrait 30 photocopies pour la sortie",
  "Pouvez-vous réparer le vidéoprojecteur de la salle 4 ?",
  "Un élève était absent ce matin, faut-il prévenir la vie scolaire ?",
];

test("repère les demandes d'absence du personnel", () => {
  for (const t of yes) assert.equal(looksLikeStaffAbsenceRequest("", t), true, t);
});

test("laisse passer les demandes ordinaires", () => {
  for (const t of no) assert.equal(looksLikeStaffAbsenceRequest("", t), false, t);
});

test("regarde aussi le sujet", () => {
  assert.equal(looksLikeStaffAbsenceRequest("Demande d'absence", "Voir pièce jointe, merci."), true);
});
