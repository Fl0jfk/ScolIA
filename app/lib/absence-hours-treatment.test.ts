import { test } from "node:test";
import assert from "node:assert/strict";
import {
  needsMakeupSlotsFromStaff,
  requiresProcessorAfterValidation,
  suggestHoursTreatmentFromPreference,
} from "./absence-hours-treatment";

test("préférence prof déclaration → préremplit rectorat (collège/lycée)", () => {
  assert.equal(
    suggestHoursTreatmentFromPreference("professeur", "Lycée La Providence", "DECLARATION_INSTANCE"),
    "DECLARATION_RECTORAT",
  );
});

test("préférence prof déclaration → préremplit ONISE (école)", () => {
  assert.equal(
    suggestHoursTreatmentFromPreference("professeur", "École Saint-Joseph", "DECLARATION_INSTANCE"),
    "DECLARATION_ONISE",
  );
});

test("préférence prof rattrapage → RATTRAPAGE_INTERNE", () => {
  assert.equal(
    suggestHoursTreatmentFromPreference("professeur", "Collège", "RATTRAPAGE_INTERNE"),
    "RATTRAPAGE_INTERNE",
  );
});

test("pas de créneaux si décision = déclaration rectorat (même avec ancienne relance)", () => {
  assert.equal(
    needsMakeupSlotsFromStaff({
      managerDecision: "VALIDEE",
      workflowStatus: "A_TRAITER",
      hoursTreatment: "DECLARATION_RECTORAT",
      staffPreferredTreatment: "DECLARATION_INSTANCE",
      makeupSlotsRelanceAt: "2026-09-11T10:00:00.000Z",
    }),
    false,
  );
});

test("pas de créneaux si préférence déclaration en attente", () => {
  assert.equal(
    needsMakeupSlotsFromStaff({
      managerDecision: "EN_ATTENTE",
      workflowStatus: "OUVERTE",
      staffPreferredTreatment: "DECLARATION_INSTANCE",
      makeupSlotsRelanceAt: "2026-09-11T10:00:00.000Z",
    }),
    false,
  );
});

test("créneaux demandés si rattrapage validé sans plage", () => {
  assert.equal(
    needsMakeupSlotsFromStaff({
      managerDecision: "VALIDEE",
      workflowStatus: "A_TRAITER",
      hoursTreatment: "RATTRAPAGE_INTERNE",
    }),
    true,
  );
});

test("processeur : uniquement déclaration rectorat pour les profs", () => {
  assert.equal(
    requiresProcessorAfterValidation({
      data: { scope: "professeur" },
      hoursTreatment: "DECLARATION_RECTORAT",
    }),
    true,
  );
  assert.equal(
    requiresProcessorAfterValidation({
      data: { scope: "professeur" },
      hoursTreatment: "RATTRAPAGE_INTERNE",
    }),
    false,
  );
  assert.equal(
    requiresProcessorAfterValidation({
      data: { scope: "ogec" },
      hoursTreatment: "RATTRAPAGE",
    }),
    true,
  );
});
