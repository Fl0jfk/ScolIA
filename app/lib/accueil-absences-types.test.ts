import { test } from "node:test";
import assert from "node:assert/strict";
import {
  absenceCoversSlot,
  asDateKey,
  canAnnulerAccueilBoardRow,
  datesOverlap,
  timesOverlap,
  type AccueilBoardRow,
} from "./accueil-absences-types";

function boardRow(overrides: Partial<AccueilBoardRow>): AccueilBoardRow {
  return {
    id: "id-1",
    kind: "eleve",
    displayName: "Test",
    subtitle: "",
    dateDebut: "2026-10-05",
    dateFin: "2026-10-05",
    heureDebut: null,
    heureFin: null,
    motif: null,
    createdByNom: null,
    source: "accueil",
    ...overrides,
  };
}

test("asDateKey normalise string et Date UTC minuit", () => {
  assert.equal(asDateKey("2026-09-02"), "2026-09-02");
  assert.equal(asDateKey("2026-09-02T00:00:00.000Z"), "2026-09-02");
  assert.equal(asDateKey(new Date(Date.UTC(2026, 8, 2))), "2026-09-02");
});

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

test("canAnnulerAccueilBoardRow — accueil élève seulement", () => {
  assert.equal(canAnnulerAccueilBoardRow(boardRow({ source: "accueil" })), true);
  assert.equal(canAnnulerAccueilBoardRow(boardRow({ source: "appel" })), false);
  assert.equal(canAnnulerAccueilBoardRow(boardRow({ kind: "professeur", source: "accueil" })), false);
});
