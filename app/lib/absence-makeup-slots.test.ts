import assert from "node:assert/strict";
import test from "node:test";
import {
  emptyMakeupSlotDraft,
  formatMakeupSlotsText,
  hasMakeupSlotsInfo,
  isRattrapageTreatment,
  needsMakeupSlotsFromStaff,
} from "./absence-hours-treatment";

test("formatMakeupSlotsText formate plusieurs créneaux en français", () => {
  const text = formatMakeupSlotsText([
    { date: "2026-03-12", startTime: "10:00", endTime: "12:00" },
    { date: "2026-03-13", startTime: "14:30", endTime: "16:00" },
  ]);
  assert.match(text, /12 mars 2026/);
  assert.match(text, /de 10h à 12h/);
  assert.match(text, /13 mars 2026/);
  assert.match(text, /de 14h30 à 16h/);
  assert.ok(text.includes(" ; "));
});

test("formatMakeupSlotsText ignore les lignes incomplètes", () => {
  assert.equal(
    formatMakeupSlotsText([
      emptyMakeupSlotDraft(),
      { date: "2026-03-12", startTime: "10:00", endTime: "" },
    ]),
    "",
  );
});

test("needsMakeupSlotsFromStaff détecte une validation rattrapage sans créneaux", () => {
  assert.equal(
    needsMakeupSlotsFromStaff({
      workflowStatus: "A_TRAITER",
      managerDecision: "VALIDEE",
      hoursTreatment: "RATTRAPAGE_INTERNE",
    }),
    true,
  );
  assert.equal(
    needsMakeupSlotsFromStaff({
      workflowStatus: "A_TRAITER",
      managerDecision: "VALIDEE",
      hoursTreatment: "RATTRAPAGE_INTERNE",
      staffPreferredMakeupSlots: "Jeudi 10h",
    }),
    false,
  );
  assert.equal(isRattrapageTreatment("DECLARATION_RECTORAT"), false);
  assert.equal(hasMakeupSlotsInfo({ directionConfirmedMakeupSlots: "  " }), false);
});
