import assert from "node:assert/strict";
import test from "node:test";
import {
  absenceMatchesPersonnel,
  absenceToLeaveSpan,
  absencesToLeaveSpans,
  isValidatedAbsenceForEdtHole,
  mergeLeaveSpans,
} from "./absences-leave-spans";
import { absenceShouldCreatePaieElement } from "./absences-paie-sync-rules";

test("absence validated + calendar → LeaveSpan", () => {
  const span = absenceToLeaveSpan({
    id: "a1",
    managerDecision: "VALIDEE",
    calendarVisible: true,
    personnelId: "p1",
    displayName: "Alice",
    createdBy: { userId: "u1" },
    data: { startDate: "2026-09-22", endDate: "2026-09-23", reason: "Maladie" },
    hoursTreatment: "MALADIE",
  });
  assert.ok(span);
  assert.equal(span!.startDate, "2026-09-22");
  assert.equal(span!.endDate, "2026-09-23");
  assert.equal(span!.type, "absence_maladie");
  assert.match(span!.label, /Maladie/);
});

test("absence en attente → pas de LeaveSpan", () => {
  assert.equal(
    absenceToLeaveSpan({
      id: "a2",
      managerDecision: "EN_ATTENTE",
      calendarVisible: false,
      personnelId: "p1",
      displayName: "Alice",
      data: { startDate: "2026-09-22", endDate: "2026-09-22", reason: "X" },
    }),
    null,
  );
  assert.equal(
    isValidatedAbsenceForEdtHole({
      id: "a2",
      managerDecision: "EN_ATTENTE",
      calendarVisible: true,
      personnelId: null,
      displayName: "Alice",
      data: { startDate: "2026-09-22", endDate: "2026-09-22" },
    }),
    false,
  );
});

test("match personnelId ou createdBy.userId", () => {
  const rec = {
    id: "a3",
    managerDecision: "VALIDEE" as const,
    calendarVisible: true,
    personnelId: "pers-9",
    displayName: "Bob",
    createdBy: { userId: "ext-1" },
    data: { startDate: "2026-09-22", endDate: "2026-09-22", reason: "Congé" },
  };
  assert.equal(absenceMatchesPersonnel(rec, { personnelId: "pers-9" }), true);
  assert.equal(absenceMatchesPersonnel(rec, { userId: "ext-1" }), true);
  assert.equal(absenceMatchesPersonnel(rec, { personnelId: "other" }), false);
  assert.equal(absenceMatchesPersonnel(rec, { userId: "other" }), false);
});

test("absencesToLeaveSpans filtre + mergeLeaveSpans déduplique", () => {
  const spans = absencesToLeaveSpans(
    [
      {
        id: "a1",
        managerDecision: "VALIDEE",
        calendarVisible: true,
        personnelId: "p1",
        displayName: "A",
        createdBy: { userId: "u1" },
        data: { startDate: "2026-09-22", endDate: "2026-09-22", reason: "Maladie" },
        hoursTreatment: "MALADIE",
      },
      {
        id: "a2",
        managerDecision: "VALIDEE",
        calendarVisible: true,
        personnelId: "p2",
        displayName: "B",
        createdBy: { userId: "u2" },
        data: { startDate: "2026-09-22", endDate: "2026-09-22", reason: "Autre" },
      },
    ],
    { personnelId: "p1", userId: "u1" },
  );
  assert.equal(spans.length, 1);

  const merged = mergeLeaveSpans(spans, spans, [
    { startDate: "2026-09-22", endDate: "2026-09-22", type: "conge", label: "Congé" },
  ]);
  assert.equal(merged.length, 2);
});

test("paie : rattrapage skip ; maladie / déduction ok", () => {
  assert.equal(absenceShouldCreatePaieElement("RATTRAPAGE"), false);
  assert.equal(absenceShouldCreatePaieElement("RATTRAPAGE_INTERNE"), false);
  assert.equal(absenceShouldCreatePaieElement("MALADIE"), true);
  assert.equal(absenceShouldCreatePaieElement("DEDUCTION_SALAIRE"), true);
  assert.equal(absenceShouldCreatePaieElement("DECLARATION_RECTORAT"), true);
  assert.equal(absenceShouldCreatePaieElement(null), true);
});
