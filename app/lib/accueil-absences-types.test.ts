import { test } from "node:test";
import assert from "node:assert/strict";
import { absenceCoversSlot, datesOverlap, timesOverlap } from "./accueil-absences-types";

test("datesOverlap inclusive", () => {
  assert.equal(datesOverlap("2026-08-30", "2026-08-30", "2026-08-30", "2026-08-30"), true);
  assert.equal(datesOverlap("2026-08-29", "2026-08-31", "2026-08-30", "2026-08-30"), true);
  assert.equal(datesOverlap("2026-08-01", "2026-08-02", "2026-08-03", "2026-08-04"), false);
});

test("timesOverlap — journée entière recouvre un créneau", () => {
  assert.equal(timesOverlap(null, null, "08:00", "09:00"), true);
  assert.equal(timesOverlap("08:00", "10:00", "09:00", "11:00"), true);
  assert.equal(timesOverlap("08:00", "09:00", "10:00", "11:00"), false);
});

test("absenceCoversSlot accueil vs appel", () => {
  assert.equal(
    absenceCoversSlot({
      dateDebut: "2026-08-30",
      dateFin: "2026-08-30",
      heureDebut: null,
      heureFin: null,
      slotDate: "2026-08-30",
      slotHeureDebut: "10:00",
      slotHeureFin: "11:00",
    }),
    true,
  );
  assert.equal(
    absenceCoversSlot({
      dateDebut: "2026-08-30",
      dateFin: "2026-08-30",
      heureDebut: "08:00",
      heureFin: "10:00",
      slotDate: "2026-08-30",
      slotHeureDebut: "10:00",
      slotHeureFin: "11:00",
    }),
    false,
  );
});
