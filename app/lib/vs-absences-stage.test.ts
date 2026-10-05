import assert from "node:assert/strict";
import test from "node:test";
import {
  isStageVsAbsenceMotif,
  isStageVsAbsenceRow,
  STAGE_VS_ABSENCE_MOTIF_PG_REGEX,
} from "./vs-absences-stage";

test("isStageVsAbsenceMotif — motifs conventions stage", () => {
  assert.equal(isStageVsAbsenceMotif("Stage — Acme Corp [abc1234567]"), true);
  assert.equal(isStageVsAbsenceMotif("stage - foo"), true);
  assert.equal(isStageVsAbsenceMotif("Maladie"), false);
  assert.equal(isStageVsAbsenceMotif(null), false);
});

test("isStageVsAbsenceRow", () => {
  assert.equal(isStageVsAbsenceRow({ motif: "Stage — X" }), true);
  assert.equal(isStageVsAbsenceRow({ motif: "RDV médical" }), false);
});

test("regex PG documentée", () => {
  assert.ok(STAGE_VS_ABSENCE_MOTIF_PG_REGEX.startsWith("^Stage"));
});
