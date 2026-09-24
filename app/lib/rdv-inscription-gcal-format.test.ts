import assert from "node:assert/strict";
import test from "node:test";
import { buildRdvInscriptionGcalSummary } from "@/app/lib/rdv-inscription-gcal-format";

test("buildRdvInscriptionGcalSummary — format INSCR", () => {
  assert.equal(
    buildRdvInscriptionGcalSummary({
      studentLastName: "Dupont",
      studentFirstName: "Marie",
      niveauLabel: "6ème",
      regime: "DP",
    }),
    "INSCR - DUPONT - Marie - 6ème - DP",
  );
  assert.equal(
    buildRdvInscriptionGcalSummary({
      studentLastName: "Martin",
      studentFirstName: "Paul",
      niveauLabel: "2nde",
      regime: "EXT",
    }),
    "INSCR - MARTIN - Paul - 2nde - EXT",
  );
  assert.equal(
    buildRdvInscriptionGcalSummary({
      studentLastName: "Bernard",
      studentFirstName: "Léa",
    }),
    "INSCR - BERNARD - Léa",
  );
});
