import { test } from "node:test";
import assert from "node:assert/strict";
import {
  addOuvrableDays,
  formatCongeExceptionnelReason,
  forcedHoursTreatmentForNonDiscretionaryAbsence,
  isNonDiscretionaryAbsence,
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

test("relance active : créneaux re-demandés même si déjà proposés", () => {
  assert.equal(
    needsMakeupSlotsFromStaff({
      managerDecision: "VALIDEE",
      workflowStatus: "CLOTUREE",
      hoursTreatment: "RATTRAPAGE_INTERNE",
      staffPreferredMakeupSlots: "Mardi 14h-16h",
      makeupSlotsRelanceAt: "2026-09-11T10:00:00.000Z",
    }),
    true,
  );
});

test("processeur : arrêt de travail / enfant malade / congé exceptionnel aussi pour les profs", () => {
  assert.equal(
    requiresProcessorAfterValidation({
      data: { scope: "professeur" },
      hoursTreatment: "MALADIE",
    }),
    true,
  );
  assert.equal(
    requiresProcessorAfterValidation({
      data: { scope: "professeur" },
      hoursTreatment: "ENFANT_MALADE",
    }),
    true,
  );
  assert.equal(
    requiresProcessorAfterValidation({
      data: { scope: "professeur" },
      hoursTreatment: "CONGE_EXCEPTIONNEL",
    }),
    true,
  );
  assert.equal(
    requiresProcessorAfterValidation({
      data: { scope: "ogec" },
      hoursTreatment: "MALADIE",
    }),
    true,
  );
});

test("préférence maladie / congé exceptionnel → traitement forcé", () => {
  assert.equal(
    suggestHoursTreatmentFromPreference("ogec", null, "MALADIE"),
    "MALADIE",
  );
  assert.equal(
    suggestHoursTreatmentFromPreference("professeur", "Lycée", "ENFANT_MALADE"),
    "ENFANT_MALADE",
  );
  assert.equal(
    suggestHoursTreatmentFromPreference("ogec", null, "CONGE_EXCEPTIONNEL"),
    "CONGE_EXCEPTIONNEL",
  );
});

test("pas de créneaux si préférence arrêt de travail / congé exceptionnel en attente", () => {
  assert.equal(
    needsMakeupSlotsFromStaff({
      managerDecision: "EN_ATTENTE",
      workflowStatus: "OUVERTE",
      staffPreferredTreatment: "MALADIE",
    }),
    false,
  );
  assert.equal(
    needsMakeupSlotsFromStaff({
      managerDecision: "EN_ATTENTE",
      workflowStatus: "OUVERTE",
      staffPreferredTreatment: "CONGE_EXCEPTIONNEL",
    }),
    false,
  );
});

test("motifs non discrétionnaires détectés (arrêt de travail, legacy, congé)", () => {
  assert.equal(
    isNonDiscretionaryAbsence({ data: { reason: "Maladie" } }),
    true,
  );
  assert.equal(
    isNonDiscretionaryAbsence({ data: { reason: "Arrêt de travail" } }),
    true,
  );
  assert.equal(
    isNonDiscretionaryAbsence({ data: { reason: "Enfant malade" } }),
    true,
  );
  assert.equal(
    isNonDiscretionaryAbsence({ data: { reason: "arrêt maladie" } }),
    true,
  );
  assert.equal(
    isNonDiscretionaryAbsence({ data: { reason: "Arrêt-maladie du 12/03" } }),
    true,
  );
  assert.equal(
    isNonDiscretionaryAbsence({ data: { reason: "arrêt de travail du 12/03" } }),
    true,
  );
  assert.equal(
    isNonDiscretionaryAbsence({ data: { reason: "enfant malade (fille)" } }),
    true,
  );
  assert.equal(
    isNonDiscretionaryAbsence({ data: { reason: "Congé exceptionnel" } }),
    true,
  );
  assert.equal(
    isNonDiscretionaryAbsence({
      data: { reason: "Congé exceptionnel — Décès du père, de la mère, beau-père, belle-mère, frère ou sœur" },
    }),
    true,
  );
  assert.equal(
    isNonDiscretionaryAbsence({ data: { reason: "Rendez-vous médical" } }),
    false,
  );
  assert.equal(
    forcedHoursTreatmentForNonDiscretionaryAbsence({ data: { reason: "Enfant malade" } }),
    "ENFANT_MALADE",
  );
  assert.equal(
    forcedHoursTreatmentForNonDiscretionaryAbsence({ data: { reason: "arrêt maladie" } }),
    "MALADIE",
  );
  assert.equal(
    forcedHoursTreatmentForNonDiscretionaryAbsence({ data: { reason: "Arrêt de travail" } }),
    "MALADIE",
  );
  assert.equal(
    forcedHoursTreatmentForNonDiscretionaryAbsence({ data: { reason: "Congé exceptionnel — Mariage ou PACS du salarié" } }),
    "CONGE_EXCEPTIONNEL",
  );
});

test("formatCongeExceptionnelReason et addOuvrableDays", () => {
  assert.equal(
    formatCongeExceptionnelReason("MARIAGE_PACS"),
    "Congé exceptionnel — Mariage ou PACS du salarié",
  );
  assert.equal(
    formatCongeExceptionnelReason("DECES_GRAND_PARENT"),
    "Congé exceptionnel — Décès d’un grand-parent (ou ascendant au-delà)",
  );
  assert.equal(
    formatCongeExceptionnelReason("AUTRE"),
    "Congé exceptionnel — Autre motif familial (à préciser)",
  );
  // Lundi 2026-03-16 + 3 ouvrables → mercredi 18
  assert.equal(addOuvrableDays("2026-03-16", 3), "2026-03-18");
  // Vendredi + 2 ouvrables → lundi suivant
  assert.equal(addOuvrableDays("2026-03-20", 2), "2026-03-23");
});

test("processeur : uniquement déclaration rectorat pour les profs (hors non discrétionnaire)", () => {
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
